//------------------------------------------------------------------------------------------------------------
// file: api.rs
// purpose: OpenAI-compatible API proxy mode for SQ
//          Triages prompts: cache → local ollama → upstream provider
//
// Usage: sq api <config.json> [port]
// Non-breaking addition — existing `sq route` tenant proxy is unchanged.
//
// v0.6.0
//------------------------------------------------------------------------------------------------------------

use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::cache::PromptCache;
use crate::triage::{self, Tier, FeedbackLoop};

// -----------------------------------------------------------------------------------------------------------
// Config
// -----------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct ApiConfig {
    pub local_url: String,           // e.g. "http://127.0.0.1:11434"
    pub local_model: String,         // e.g. "qwen3-coder-next:latest"
    pub local_timeout_secs: u64,
    pub upstream_url: String,        // e.g. "https://api.anthropic.com"
    pub upstream_api_key: String,
    pub upstream_model: String,      // e.g. "claude-opus-4-6"
    pub cache_max_entries: usize,
    pub cache_ttl_secs: u64,
    pub signal_threshold: usize,
    pub escalation_threshold: f64,
    pub feedback_window: usize,
}

impl Default for ApiConfig {
    fn default() -> Self {
        ApiConfig {
            local_url: "http://127.0.0.1:11434".into(),
            local_model: "qwen3-coder-next:latest".into(),
            local_timeout_secs: 120,
            upstream_url: "https://api.anthropic.com".into(),
            upstream_api_key: String::new(),
            upstream_model: "claude-sonnet-4-6".into(),
            cache_max_entries: 512,
            cache_ttl_secs: 3600,
            signal_threshold: 1,
            escalation_threshold: 0.25,
            feedback_window: 100,
        }
    }
}

pub fn load_api_config(path: &str) -> Result<ApiConfig, Box<dyn std::error::Error>> {
    let contents = std::fs::read_to_string(path)?;
    let config: ApiConfig = serde_json::from_str(&contents)?;
    Ok(config)
}

// -----------------------------------------------------------------------------------------------------------
// OpenAI-compatible request/response types (minimal subset)
// -----------------------------------------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatRequest {
    model: Option<String>,
    messages: Vec<ChatMessage>,
    #[serde(default)]
    max_tokens: Option<u32>,
}

#[derive(Debug, Serialize)]
struct ChatChoice {
    index: u32,
    message: ChatResponseMessage,
    finish_reason: String,
}

#[derive(Debug, Serialize)]
struct ChatResponseMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct ChatResponse {
    id: String,
    object: String,
    model: String,
    choices: Vec<ChatChoice>,
    usage: ChatUsage,
}

#[derive(Debug, Serialize)]
struct ChatUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

fn make_chat_response(content: &str, model: &str) -> String {
    let resp = ChatResponse {
        id: format!("sq-{}", Instant::now().elapsed().as_nanos()),
        object: "chat.completion".into(),
        model: model.into(),
        choices: vec![ChatChoice {
            index: 0,
            message: ChatResponseMessage {
                role: "assistant".into(),
                content: content.into(),
            },
            finish_reason: "stop".into(),
        }],
        usage: ChatUsage {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
        },
    };
    serde_json::to_string(&resp).unwrap_or_default()
}

// -----------------------------------------------------------------------------------------------------------
// HTTP helpers (reuse patterns from main.rs / router.rs)
// -----------------------------------------------------------------------------------------------------------

fn read_request(stream: &mut TcpStream) -> Result<(String, String), Box<dyn std::error::Error>> {
    let mut buf = vec![0u8; 65536];
    let mut total = 0;

    loop {
        let n = stream.read(&mut buf[total..])?;
        if n == 0 { break; }
        total += n;

        // Find header end
        if let Some(pos) = find_header_end(&buf[..total]) {
            let header = String::from_utf8_lossy(&buf[..pos]).to_string();
            let content_length = extract_content_length(&header);

            let body_start = pos + 4; // skip \r\n\r\n
            let mut body_bytes = buf[body_start..total].to_vec();

            // Read remaining body if needed
            while body_bytes.len() < content_length {
                let mut extra = vec![0u8; content_length - body_bytes.len()];
                let n = stream.read(&mut extra)?;
                if n == 0 { break; }
                body_bytes.extend_from_slice(&extra[..n]);
            }

            let body = String::from_utf8_lossy(&body_bytes[..content_length.min(body_bytes.len())]).to_string();
            return Ok((header, body));
        }

        if total >= 65536 { return Err("Header too large".into()); }
    }
    Err("Connection closed".into())
}

fn find_header_end(buf: &[u8]) -> Option<usize> {
    for i in 0..buf.len().saturating_sub(3) {
        if &buf[i..i+4] == b"\r\n\r\n" {
            return Some(i);
        }
    }
    None
}

fn extract_content_length(header: &str) -> usize {
    for line in header.lines() {
        if line.to_lowercase().starts_with("content-length:") {
            if let Some(v) = line.split(':').nth(1) {
                return v.trim().parse().unwrap_or(0);
            }
        }
    }
    0
}

fn send_json_response(stream: &mut TcpStream, status: u16, body: &str) {
    let status_text = match status {
        200 => "OK",
        400 => "Bad Request",
        500 => "Internal Server Error",
        _ => "Error",
    };
    let response = format!(
        "HTTP/1.1 {} {}\r\n\
         Content-Type: application/json\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Content-Length: {}\r\n\r\n{}",
        status, status_text, body.len(), body
    );
    let _ = stream.write_all(response.as_bytes());
}

fn send_cors_preflight(stream: &mut TcpStream) {
    let response = "HTTP/1.1 204 No Content\r\n\
        Access-Control-Allow-Origin: *\r\n\
        Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\
        Access-Control-Allow-Headers: Authorization, Content-Type\r\n\
        Access-Control-Max-Age: 86400\r\n\r\n";
    let _ = stream.write_all(response.as_bytes());
}

// -----------------------------------------------------------------------------------------------------------
// Proxy to ollama (local tier)
// -----------------------------------------------------------------------------------------------------------

fn proxy_to_local(config: &ApiConfig, request_body: &str) -> Result<String, Box<dyn std::error::Error>> {
    let url = format!("{}/v1/chat/completions", config.local_url);

    // Override model in the request, preserve full message history
    let mut parsed: serde_json::Value = serde_json::from_str(request_body)?;
    parsed["model"] = serde_json::Value::String(config.local_model.clone());
    parsed["stream"] = serde_json::Value::Bool(false);
    let body_str = parsed.to_string();

    // Parse host/port from URL
    let url_trimmed = url.trim_start_matches("http://").trim_start_matches("https://");
    let (host_port, path) = url_trimmed.split_once('/').unwrap_or((url_trimmed, "v1/chat/completions"));

    let mut stream = TcpStream::connect(host_port)?;
    stream.set_read_timeout(Some(Duration::from_secs(config.local_timeout_secs)))?;
    stream.set_write_timeout(Some(Duration::from_secs(30)))?;

    let request = format!(
        "POST /{} HTTP/1.1\r\n\
         Host: {}\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\r\n{}",
        path, host_port, body_str.len(), body_str
    );
    stream.write_all(request.as_bytes())?;

    let mut response = Vec::new();
    stream.read_to_end(&mut response)?;
    let response_str = String::from_utf8_lossy(&response).to_string();

    // Extract body from HTTP response
    if let Some(pos) = response_str.find("\r\n\r\n") {
        let body = &response_str[pos+4..];
        // Extract content from OpenAI-format response
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(body) {
            if let Some(content) = parsed["choices"][0]["message"]["content"].as_str() {
                return Ok(content.to_string());
            }
        }
        return Ok(body.to_string());
    }

    Err("No response body".into())
}

// -----------------------------------------------------------------------------------------------------------
// Proxy to upstream (upstream tier)
// -----------------------------------------------------------------------------------------------------------

fn proxy_to_upstream(config: &ApiConfig, request_body: &str) -> Result<String, Box<dyn std::error::Error>> {
    let url_trimmed = config.upstream_url.trim_start_matches("http://").trim_start_matches("https://");
    let (host_port, _) = url_trimmed.split_once('/').unwrap_or((url_trimmed, ""));

    // Override model in the request, preserve full message history
    let mut parsed: serde_json::Value = serde_json::from_str(request_body)?;
    parsed["model"] = serde_json::Value::String(config.upstream_model.clone());
    parsed["stream"] = serde_json::Value::Bool(false);
    let body_str = parsed.to_string();

    let mut stream = TcpStream::connect(host_port)?;
    stream.set_read_timeout(Some(Duration::from_secs(300)))?;
    stream.set_write_timeout(Some(Duration::from_secs(30)))?;

    let request = format!(
        "POST /v1/chat/completions HTTP/1.1\r\n\
         Host: {}\r\n\
         Content-Type: application/json\r\n\
         Authorization: Bearer {}\r\n\
         Content-Length: {}\r\n\r\n{}",
        host_port, config.upstream_api_key, body_str.len(), body_str
    );
    stream.write_all(request.as_bytes())?;

    let mut response = Vec::new();
    stream.read_to_end(&mut response)?;
    let response_str = String::from_utf8_lossy(&response).to_string();

    if let Some(pos) = response_str.find("\r\n\r\n") {
        let body = &response_str[pos+4..];
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(body) {
            if let Some(content) = parsed["choices"][0]["message"]["content"].as_str() {
                return Ok(content.to_string());
            }
        }
        return Ok(body.to_string());
    }

    Err("No response body from upstream".into())
}

// -----------------------------------------------------------------------------------------------------------
// Extract the last user message (for triage scoring) and full request body (for proxying)
// -----------------------------------------------------------------------------------------------------------

fn extract_last_user_message(body: &str) -> Option<String> {
    let req: ChatRequest = serde_json::from_str(body).ok()?;
    req.messages.iter().rev()
        .find(|m| m.role == "user")
        .map(|m| m.content.clone())
}

/// Parse the request body, validate it has messages, return the full body for proxying
fn validate_request(body: &str) -> Option<String> {
    let req: ChatRequest = serde_json::from_str(body).ok()?;
    if req.messages.is_empty() { return None; }
    Some(body.to_string())
}

// -----------------------------------------------------------------------------------------------------------
// Main API proxy server
// -----------------------------------------------------------------------------------------------------------

pub fn run_api(config_path: &str, listen_port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let config = load_api_config(config_path)?;
    let cache = Arc::new(Mutex::new(PromptCache::new(config.cache_max_entries, config.cache_ttl_secs)));
    let feedback = Arc::new(Mutex::new(FeedbackLoop::new(config.feedback_window)));
    let config = Arc::new(config);

    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║             SQ API Proxy v0.6.0                         ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();
    println!("Listening on: 0.0.0.0:{}", listen_port);
    println!("Local model:  {} @ {}", config.local_model, config.local_url);
    println!("Upstream:     {}", config.upstream_url);
    println!("Cache:        {} entries, {} sec TTL", config.cache_max_entries, config.cache_ttl_secs);
    println!();

    let listener = TcpListener::bind(format!("0.0.0.0:{}", listen_port))?;

    for stream in listener.incoming() {
        match stream {
            Ok(mut client) => {
                let config = Arc::clone(&config);
                let cache = Arc::clone(&cache);
                let feedback = Arc::clone(&feedback);

                std::thread::spawn(move || {
                    let _ = client.set_read_timeout(Some(Duration::from_secs(30)));
                    let _ = client.set_write_timeout(Some(Duration::from_secs(30)));

                    let (header, body) = match read_request(&mut client) {
                        Ok(r) => r,
                        Err(_) => return,
                    };

                    // CORS preflight
                    if header.starts_with("OPTIONS ") {
                        send_cors_preflight(&mut client);
                        return;
                    }

                    // Stats endpoint
                    if header.starts_with("GET /stats") {
                        let c = cache.lock().unwrap();
                        let (hits, misses, size) = c.stats();
                        let fl = feedback.lock().unwrap();
                        let stats = serde_json::json!({
                            "cache_hits": hits,
                            "cache_misses": misses,
                            "cache_size": size,
                            "cache_hit_rate": c.hit_rate(),
                            "local_failure_rate": fl.failure_rate(),
                        });
                        send_json_response(&mut client, 200, &stats.to_string());
                        return;
                    }

                    // Only handle POST /v1/chat/completions
                    if !header.starts_with("POST ") {
                        send_json_response(&mut client, 400, r#"{"error":"Only POST /v1/chat/completions supported"}"#);
                        return;
                    }

                    // Validate request has messages
                    let request_body = match validate_request(&body) {
                        Some(b) => b,
                        None => {
                            send_json_response(&mut client, 400, r#"{"error":"Invalid request or no messages"}"#);
                            return;
                        }
                    };

                    // Extract last user message for triage scoring + cache key
                    let prompt = match extract_last_user_message(&body) {
                        Some(p) => p,
                        None => {
                            // No user message — pass through to upstream as-is
                            match proxy_to_upstream(&config, &request_body) {
                                Ok(content) => {
                                    let resp = make_chat_response(&content, &config.upstream_model);
                                    send_json_response(&mut client, 200, &resp);
                                }
                                Err(e) => {
                                    let err = serde_json::json!({"error": format!("{}", e)});
                                    send_json_response(&mut client, 500, &err.to_string());
                                }
                            }
                            return;
                        }
                    };

                    // 1. Check static patterns (only for single-turn)
                    if let Some(static_resp) = PromptCache::check_static(&prompt) {
                        let req: Option<ChatRequest> = serde_json::from_str(&body).ok();
                        let is_single_turn = req.map(|r| r.messages.len() <= 1).unwrap_or(false);
                        if is_single_turn {
                            let resp = make_chat_response(static_resp, "sq-cache");
                            send_json_response(&mut client, 200, &resp);
                            return;
                        }
                    }

                    // 2. Check cache (keyed on last user message — only for short conversations)
                    {
                        let mut c = cache.lock().unwrap();
                        if let Some(cached) = c.get(&prompt) {
                            let resp = make_chat_response(&cached, "sq-cache");
                            send_json_response(&mut client, 200, &resp);
                            return;
                        }
                    }

                    // 3. Triage (score based on last user message)
                    let should_escalate = feedback.lock().unwrap().should_escalate(config.escalation_threshold);
                    let mut decision = triage::evaluate(&prompt, config.signal_threshold);

                    // Auto-escalate if local model is failing too much
                    if decision.tier == Tier::Local && should_escalate {
                        decision.tier = Tier::Upstream;
                        decision.reason = format!("escalated: {}", decision.reason);
                    }

                    println!("[triage] {} → {:?} ({})", &prompt[..prompt.len().min(60)], decision.tier, decision.reason);

                    // 4. Dispatch — full message history forwarded to backend
                    let result = match decision.tier {
                        Tier::Cache => unreachable!(), // handled above
                        Tier::Local => {
                            match proxy_to_local(&config, &request_body) {
                                Ok(resp) => {
                                    feedback.lock().unwrap().record(true);
                                    Ok(resp)
                                }
                                Err(e) => {
                                    feedback.lock().unwrap().record(false);
                                    eprintln!("[local error] {} — escalating to upstream", e);
                                    proxy_to_upstream(&config, &request_body)
                                }
                            }
                        }
                        Tier::Upstream => proxy_to_upstream(&config, &request_body),
                    };

                    match result {
                        Ok(content) => {
                            // Cache the response (keyed on last user message)
                            cache.lock().unwrap().set(&prompt, &content);
                            let model = if decision.tier == Tier::Local {
                                config.local_model.as_str()
                            } else {
                                config.upstream_model.as_str()
                            };
                            let resp = make_chat_response(&content, model);
                            send_json_response(&mut client, 200, &resp);
                        }
                        Err(e) => {
                            let err = serde_json::json!({"error": format!("{}", e)});
                            send_json_response(&mut client, 500, &err.to_string());
                        }
                    }
                });
            }
            Err(e) => eprintln!("Connection error: {}", e),
        }
    }

    Ok(())
}
