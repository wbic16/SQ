//------------------------------------------------------------------------------------------------------------
// file: router.rs
// purpose: Token-based routing layer for multi-tenant SQ deployments
// 
// Architecture:
// - Single router process listens on public port (e.g., 443 or 1337)
// - Each tenant gets dedicated SQ instance on private port with --key and --data-dir
// - Router reads Authorization header, looks up tenant config, proxies to backend
//
// Usage: sq route <config.json> <port>
//------------------------------------------------------------------------------------------------------------

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, RwLock};
use std::time::Duration;

const MAX_HEADER_SIZE: usize = 16_384; // 16 KB header limit
const ROUTER_TIMEOUT_MS: u64 = 30_000; // 30 second timeout

// -----------------------------------------------------------------------------------------------------------
// Configuration structures
// -----------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantConfig {
    pub token: String,      // pmb-v1-xxx auth token
    pub port: u16,          // backend SQ instance port
    pub data_dir: String,   // tenant data directory
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterConfig {
    pub tenants: Vec<TenantConfig>,
}

// -----------------------------------------------------------------------------------------------------------
// Loads router configuration from JSON file
// -----------------------------------------------------------------------------------------------------------
pub fn load_router_config(path: &str) -> Result<RouterConfig, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let config: RouterConfig = serde_json::from_str(&contents)?;
    
    // Validate no duplicate tokens
    let mut seen_tokens = std::collections::HashSet::new();
    for tenant in &config.tenants {
        if !seen_tokens.insert(&tenant.token) {
            return Err(format!("Duplicate token in config: {}", tenant.token).into());
        }
    }
    
    Ok(config)
}

// -----------------------------------------------------------------------------------------------------------
// Extracts Authorization header from HTTP request
// Returns the token (without "Bearer " prefix if present)
// -----------------------------------------------------------------------------------------------------------
fn extract_auth_token(header: &str) -> Option<String> {
    for line in header.lines() {
        let lower_line = line.to_lowercase();
        if lower_line.starts_with("authorization:") {
            if let Some(value) = line.split(':').nth(1) {
                let token = value.trim();
                // Strip "Bearer " prefix if present
                if token.to_lowercase().starts_with("bearer ") {
                    return Some(token[7..].trim().to_string());
                } else {
                    return Some(token.to_string());
                }
            }
        }
    }
    None
}

// -----------------------------------------------------------------------------------------------------------
// Reads HTTP request header from stream (up to first \r\n\r\n)
// Returns (header_string, total_bytes_read)
// -----------------------------------------------------------------------------------------------------------
fn read_http_header(stream: &mut TcpStream) -> Result<(String, usize), Box<dyn std::error::Error>> {
    let mut buffer = vec![0u8; MAX_HEADER_SIZE];
    let mut total_read = 0;
    
    loop {
        let bytes_read = stream.read(&mut buffer[total_read..])?;
        if bytes_read == 0 {
            return Err("Connection closed before header complete".into());
        }
        total_read += bytes_read;
        
        // Check for end of header (\r\n\r\n)
        if total_read >= 4 {
            let check_start = if total_read > 4 { total_read - 4 } else { 0 };
            for i in check_start..total_read - 3 {
                if &buffer[i..i+4] == b"\r\n\r\n" {
                    let header = String::from_utf8_lossy(&buffer[..i+4]).to_string();
                    return Ok((header, i + 4));
                }
            }
        }
        
        if total_read >= MAX_HEADER_SIZE {
            return Err("Header too large".into());
        }
    }
}

// -----------------------------------------------------------------------------------------------------------
// Proxies HTTP request to backend SQ instance
// Returns response or error
// -----------------------------------------------------------------------------------------------------------
fn proxy_request(
    client_stream: &mut TcpStream,
    backend_port: u16,
    header: &str,
    _header_bytes: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    // Connect to backend
    let mut backend = TcpStream::connect(format!("127.0.0.1:{}", backend_port))?;
    backend.set_read_timeout(Some(Duration::from_millis(ROUTER_TIMEOUT_MS)))?;
    backend.set_write_timeout(Some(Duration::from_millis(ROUTER_TIMEOUT_MS)))?;
    
    // Forward header
    backend.write_all(header.as_bytes())?;
    
    // Check if request has body (Content-Length header)
    let content_length = extract_content_length(header);
    
    if content_length > 0 {
        // Forward body in chunks
        let mut remaining = content_length;
        let mut buffer = vec![0u8; 8192];
        
        while remaining > 0 {
            let to_read = std::cmp::min(remaining, buffer.len());
            let bytes_read = client_stream.read(&mut buffer[..to_read])?;
            if bytes_read == 0 {
                return Err("Connection closed during body transfer".into());
            }
            backend.write_all(&buffer[..bytes_read])?;
            remaining -= bytes_read;
        }
    }
    
    // Read response from backend and forward to client
    let mut response_buffer = vec![0u8; 8192];
    loop {
        match backend.read(&mut response_buffer) {
            Ok(0) => break, // EOF
            Ok(n) => {
                client_stream.write_all(&response_buffer[..n])?;
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
            Err(e) => return Err(e.into()),
        }
    }
    
    Ok(())
}

// -----------------------------------------------------------------------------------------------------------
// Extracts Content-Length header value
// -----------------------------------------------------------------------------------------------------------
fn extract_content_length(header: &str) -> usize {
    for line in header.lines() {
        let lower_line = line.to_lowercase();
        if lower_line.starts_with("content-length:") {
            if let Some(value) = line.split(':').nth(1) {
                return value.trim().parse().unwrap_or(0);
            }
        }
    }
    0
}

// -----------------------------------------------------------------------------------------------------------
// Sends error response to client
// -----------------------------------------------------------------------------------------------------------
fn send_error(stream: &mut TcpStream, code: u16, message: &str) {
    let body = format!("{{\"error\": \"{}\"}}", message);
    let response = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        code,
        match code {
            401 => "Unauthorized",
            404 => "Not Found",
            500 => "Internal Server Error",
            502 => "Bad Gateway",
            _ => "Error",
        },
        body.len(),
        body
    );
    let _ = stream.write_all(response.as_bytes());
}

// -----------------------------------------------------------------------------------------------------------
// Main router loop - listens for connections and routes to backends
// -----------------------------------------------------------------------------------------------------------
pub fn run_router(config_path: &str, listen_port: u16) -> Result<(), Box<dyn std::error::Error>> {
    // Load config
    let config = load_router_config(config_path)?;
    
    // Build token→port lookup map
    let mut token_map: HashMap<String, u16> = HashMap::new();
    for tenant in &config.tenants {
        token_map.insert(tenant.token.clone(), tenant.port);
    }
    
    let token_map = Arc::new(RwLock::new(token_map));
    
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║             SQ Router v0.5.5 (Token-based)              ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();
    println!("Listening on: 0.0.0.0:{}", listen_port);
    println!("Tenants configured: {}", config.tenants.len());
    println!("Config file: {}", config_path);
    println!();
    
    // Start listening
    let listener = TcpListener::bind(format!("0.0.0.0:{}", listen_port))?;
    
    let mut connection_id = 0u64;
    for stream in listener.incoming() {
        match stream {
            Ok(mut client_stream) => {
                connection_id += 1;
                let conn_id = connection_id;
                
                // Set timeouts
                let _ = client_stream.set_read_timeout(Some(Duration::from_millis(ROUTER_TIMEOUT_MS)));
                let _ = client_stream.set_write_timeout(Some(Duration::from_millis(ROUTER_TIMEOUT_MS)));
                
                // Read request header
                let (header, _header_bytes) = match read_http_header(&mut client_stream) {
                    Ok(h) => h,
                    Err(e) => {
                        eprintln!("[{}] Failed to read header: {}", conn_id, e);
                        send_error(&mut client_stream, 400, "Bad Request");
                        continue;
                    }
                };
                
                // Extract auth token
                let token = match extract_auth_token(&header) {
                    Some(t) => t,
                    None => {
                        eprintln!("[{}] No Authorization header", conn_id);
                        send_error(&mut client_stream, 401, "Unauthorized - No token provided");
                        continue;
                    }
                };
                
                // Look up backend port
                let backend_port = {
                    let map = token_map.read().unwrap();
                    match map.get(&token) {
                        Some(port) => *port,
                        None => {
                            eprintln!("[{}] Invalid token: {}", conn_id, &token[..8.min(token.len())]);
                            send_error(&mut client_stream, 401, "Unauthorized - Invalid token");
                            continue;
                        }
                    }
                };
                
                println!("[{}] Routing to backend port {}", conn_id, backend_port);
                
                // Proxy request
                if let Err(e) = proxy_request(&mut client_stream, backend_port, &header, _header_bytes) {
                    eprintln!("[{}] Proxy error: {}", conn_id, e);
                    send_error(&mut client_stream, 502, "Bad Gateway");
                }
            }
            Err(e) => {
                eprintln!("Connection error: {}", e);
            }
        }
    }
    
    Ok(())
}
