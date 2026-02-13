//------------------------------------------------------------------------------------------------------------
// file: main.rs
// purpose: provides primary program logic for sq - determining daemon mode vs listening mode
//
// v0.5.3 - Memory pressure fixes
//   - Fixed: unbounded thread spawning (now capped at MAX_CONCURRENT_CONNECTIONS)
//   - Fixed: no read/write timeout on TCP streams (slowloris OOM vector)
//   - Fixed: Content-Length trusted unconditionally (now capped at MAX_BODY_SIZE)
//   - Fixed: fetch_source double-allocation on truncation (now uses in-place truncate)
//   - Fixed: clone-to-serialize in every read path (new implode_ref borrows instead)
//   - Fixed: mutation flush cloned entire map (now uses implode_ref)
// v0.5.2 - Stability patch for mirrorborn.us / SQ Cloud
//   - Fixed: server-killing panics in HTTP handler (5 unwrap sites)
//   - Fixed: unconditional disk write on every request (now mutation-only)
//   - Fixed: no 404 routing (unmatched URLs created ghost .phext files)
//   - Fixed: help/version crash on Windows (bypassed shared memory for info commands)
//   - Added: thread-per-connection for concurrent request handling
//   - Added: CORS headers + OPTIONS preflight
//   - Added: graceful error handling throughout HTTP path
//   - Removed: debug printlns (WTF, Algo)
//------------------------------------------------------------------------------------------------------------

use libphext::phext;
use raw_sync::{events::*, Timeout};
use shared_memory::*;
use std::env;
use std::fs;
use std::path::Path;
use std::net::TcpListener;
use std::net::TcpStream;
use std::io::Read;
use std::io::Write;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

mod sq;
mod tests;
mod mesh;
mod router;
mod config;

const SHARED_SEGMENT_SIZE: usize = 1024*1024*1024; // 1 GB limit
const MAX_BUFFER_SIZE: usize = SHARED_SEGMENT_SIZE/2;
const WORK_SEGMENT_SIZE: usize = 1024;
const ABSURD_HEADER_SIZE: usize = 65536;

const SHARED_NAME: &str = ".sq/link";
const WORK_NAME: &str = ".sq/work";

// -----------------------------------------------------------------------------------------------------------
// Memory-pressure guardrails
// -----------------------------------------------------------------------------------------------------------
const MAX_CONCURRENT_CONNECTIONS: usize = 512;     // ~4 GB stack ceiling at 8 MB/thread
const READ_TIMEOUT_SECS: u64 = 32;                 // kill idle/slowloris connections
const WRITE_TIMEOUT_SECS: u64 = 32;
pub const MAX_BODY_SIZE: usize = 64 * 1024 * 1024; // 64 MB per request body

static ACTIVE_CONNECTIONS: AtomicUsize = AtomicUsize::new(0);

// -----------------------------------------------------------------------------------------------------------
// Shared state for the HTTP listener, protected by a mutex for thread safety
// -----------------------------------------------------------------------------------------------------------
struct ServerState {
    loaded_phext: String,
    loaded_map: HashMap<phext::Coordinate, String>,
}

// -----------------------------------------------------------------------------------------------------------
// Extracts a named header value from an HTTP request header block
// -----------------------------------------------------------------------------------------------------------
fn extract_header<'a>(header: &'a str, name: &str) -> Option<String> {
    let lower_name = name.to_lowercase();
    for line in header.lines() {
        let lower_line = line.to_lowercase();
        if lower_line.starts_with(&lower_name) {
            if let Some(value) = line.split(':').nth(1) {
                return Some(value.trim().to_string());
            }
        }
    }
    None
}

// -----------------------------------------------------------------------------------------------------------
// Validates an API key against the expected key for this tenant instance
// Returns true if auth is disabled (no key configured) or if key matches
// -----------------------------------------------------------------------------------------------------------
fn validate_auth(header: &str, expected_key: &Option<String>) -> bool {
    match expected_key {
        None => true,
        Some(key) => {
            match extract_header(header, "authorization") {
                Some(provided) => {
                    let provided = provided.trim();
                    let token = if provided.to_lowercase().starts_with("bearer ") {
                        provided[7..].trim()
                    } else {
                        provided
                    };
                    token == key
                }
                None => false,
            }
        }
    }
}

// -----------------------------------------------------------------------------------------------------------
// Sends an HTTP response with status code, CORS headers, and body
// -----------------------------------------------------------------------------------------------------------
fn send_response(stream: &mut TcpStream, status: u16, body: &str) {
    let status_text = match status {
        200 => "OK",
        204 => "No Content",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        413 => "Payload Too Large",
        500 => "Internal Server Error",
        503 => "Service Unavailable",
        _ => "OK",
    };
    let response = format!(
        "HTTP/1.1 {} {}\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\n\r\n{}",
        status, status_text, body.len(), body
    );
    let _ = stream.write_all(response.as_bytes());
}

// -----------------------------------------------------------------------------------------------------------
// Validates that a phext filename stays within the tenant data directory
// Prevents path traversal attacks (e.g., ../../etc/passwd)
// -----------------------------------------------------------------------------------------------------------
fn validate_tenant_path(phext_name: &str, data_dir: &Option<String>) -> Option<String> {
    match data_dir {
        None => Some(format!("{}.phext", phext_name)),
        Some(dir) => {
            if phext_name.contains("..") || phext_name.contains('/') || phext_name.contains('\\') {
                return None;
            }
            let path = format!("{}/{}.phext", dir, phext_name);
            Some(path)
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum HashAlgorithm {
    Xor,
    Checksum,
}

// -----------------------------------------------------------------------------------------------------------
// Loads + explodes a source phext from disk into memory
// -----------------------------------------------------------------------------------------------------------
fn fetch_source(filename: String) -> HashMap::<phext::Coordinate, String> {
    let exists = std::path::Path::new(&filename).exists();
    if exists == false {
        let _ = std::fs::write(filename.clone(), "");
    }
    let mut buffer: String = match std::fs::read_to_string(&filename) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("Failed to read {}: {}", filename, e);
            String::new()
        }
    };

    if buffer.len() > MAX_BUFFER_SIZE {
        // in-place truncation: avoids allocating a second 512 MB string
        buffer.truncate(MAX_BUFFER_SIZE);
    }
    return phext::explode(&buffer);
}

// -----------------------------------------------------------------------------------------------------------
// attempts to remove and re-create the .sq folder
// -----------------------------------------------------------------------------------------------------------
fn recreate_sq_work_files() {
    let _ = std::fs::remove_dir_all(".sq");
    let _ = std::fs::create_dir(".sq");
}

// -----------------------------------------------------------------------------------------------------------
// Creates a 1 GB shared memory segment available at .sq/link
// -----------------------------------------------------------------------------------------------------------
fn create_shared_segment() -> Result<Shmem, ShmemError> {
    ShmemConf::new().size(SHARED_SEGMENT_SIZE).flink(SHARED_NAME).create()
}

// -----------------------------------------------------------------------------------------------------------
// Creates a 1 KB shared memory segment available at .sq/work
// -----------------------------------------------------------------------------------------------------------
fn create_work_segment() -> Result<Shmem, ShmemError> {
    ShmemConf::new().size(WORK_SEGMENT_SIZE).flink(WORK_NAME).create()
}

fn is_basic_or_share(command: String) -> bool {
    return command == "share" || command == "basic";
}

// -----------------------------------------------------------------------------------------------------------
// Returns true if this command is handled locally without IPC
// -----------------------------------------------------------------------------------------------------------
fn is_local_command(command: &str) -> bool {
    command == "help" || command == "version"
}

// -----------------------------------------------------------------------------------------------------------
// Returns true if this REST command mutates the phext (requires disk write)
// -----------------------------------------------------------------------------------------------------------
fn is_mutation(command: &str) -> bool {
    command == "insert" || command == "update" || command == "delete" ||
    command == "push" || command == "slurp"
}

// -----------------------------------------------------------------------------------------------------------
// sq program loop
// -----------------------------------------------------------------------------------------------------------
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sq_exists = std::path::Path::new(".sq").exists();
    if sq_exists == false {
        let _ = std::fs::create_dir(".sq");
    }

    let command = env::args().nth(1).unwrap_or("".to_string());
    let phext_or_port = env::args().nth(2).unwrap_or("".to_string());
    let exists = std::path::Path::new(&phext_or_port).exists();
    let is_port_number = phext_or_port.parse::<u16>().is_ok();

    // -----------------------------------------------------------------------
    // Local commands: handle without IPC (fixes Windows "Failed to open event" crash)
    // -----------------------------------------------------------------------
    if is_local_command(&command) {
        let mut scroll = String::new();
        let mut empty_map: HashMap<phext::Coordinate, String> = Default::default();
        let _ = sq::process(
            0, String::new(), &mut scroll, command.clone(),
            &mut empty_map, phext::to_coordinate("1.1.1/1.1.1/1.1.1"),
            String::new(), String::new(), HashAlgorithm::Xor, 100,
        );
        println!("{}", scroll);
        return Ok(());
    }

    // Route command: sq route <config.json> <listen-port>
    if command == "route" {
        let config_path = env::args().nth(2).unwrap_or("router-config.json".to_string());
        let listen_port: u16 = env::args().nth(3)
            .and_then(|s| s.parse().ok())
            .unwrap_or(1337);
        
        return router::run_router(&config_path, listen_port);
    }

    // -----------------------------------------------------------------------
    // Listening mode: REST API server with bounded thread pool
    // -----------------------------------------------------------------------
    if command == "host" && exists == false && phext_or_port.len() > 0 && is_port_number {
        let port = phext_or_port;

        // Parse optional auth, data-dir, mesh-config, and config arguments
        // Usage: sq host <port> [--config <tenants.json>] OR [--key <pmb-v1-...>] [--data-dir <path>] [--mesh-config <path>]
        let args: Vec<String> = env::args().collect();
        
        // Check for --config (multi-tenant mode)
        let config_idx = args.iter().position(|s| s == "--config");
        if let Some(idx) = config_idx {
            if idx + 1 < args.len() {
                let config_path = &args[idx + 1];
                return run_multi_tenant_server(&port, config_path);
            } else {
                eprintln!("Error: --config requires a path argument");
                eprintln!("Usage: sq host <port> --config <tenants.json>");
                std::process::exit(1);
            }
        }
        
        // Single-tenant mode (backward compatible)
        let mut auth_key: Option<String> = None;
        let mut data_dir: Option<String> = None;
        let mut mesh_config_path: Option<String> = None;
        let mut tenant_config_path: Option<String> = None;
        let mut i = 3;
        while i < args.len() {
            match args[i].as_str() {
                "--key" => {
                    if i + 1 < args.len() {
                        auth_key = Some(args[i + 1].clone());
                        i += 2;
                    } else { i += 1; }
                }
                "--data-dir" => {
                    if i + 1 < args.len() {
                        let dir = args[i + 1].clone();
                        let _ = std::fs::create_dir_all(&dir);
                        data_dir = Some(dir);
                        i += 2;
                    } else { i += 1; }
                }
                "--mesh-config" => {
                    if i + 1 < args.len() {
                        mesh_config_path = Some(args[i + 1].clone());
                        i += 2;
                    } else { i += 1; }
                }
                "--config" => {
                    if i + 1 < args.len() {
                        tenant_config_path = Some(args[i + 1].clone());
                        i += 2;
                    } else { i += 1; }
                }
                _ => { i += 1; }
            }
        }

        // Load mesh config if provided
        let _mesh_config: Option<mesh::MeshConfig> = match mesh_config_path {
            Some(path) => {
                match mesh::load_mesh_config(&path) {
                    Ok(config) => {
                        println!("╔══════════════════════════════════════════════════════════╗");
                        println!("║               SQ Mesh Mode Enabled                       ║");
                        println!("╚══════════════════════════════════════════════════════════╝");
                        println!();
                        mesh::print_config_summary(&config);
                        
                        // Override auth_key and data_dir from mesh config if not specified
                        if auth_key.is_none() && config.inbound.enabled {
                            auth_key = Some(config.inbound.auth_key.clone());
                        }
                        if data_dir.is_none() && config.inbound.enabled {
                            data_dir = Some(config.inbound.data_dir.clone());
                        }
                        
                        Some(config)
                    }
                    Err(e) => {
                        eprintln!("❌ Failed to load mesh config: {}", e);
                        eprintln!("   Continuing in standalone mode...");
                        println!();
                        None
                    }
                }
            }
            None => None
        };
        
        // Load multi-tenant config if provided
        let tenant_map: Option<Arc<HashMap<String, config::TenantConfig>>> = match tenant_config_path {
            Some(ref path) => {
                match config::load_config(path) {
                    Ok(cfg) => {
                        println!("╔══════════════════════════════════════════════════════════╗");
                        println!("║            SQ Multi-Tenant Mode Enabled                  ║");
                        println!("╚══════════════════════════════════════════════════════════╝");
                        println!();
                        println!("Tenants configured: {}", cfg.tenants.len());
                        // Create data dirs for all tenants
                        for (token, tenant) in &cfg.tenants {
                            let _ = std::fs::create_dir_all(&tenant.data_dir);
                            println!("  {} → {} (token: {}...)", tenant.name, tenant.data_dir, &token[..8.min(token.len())]);
                        }
                        println!();
                        Some(Arc::new(cfg.tenants))
                    }
                    Err(e) => {
                        eprintln!("❌ Failed to load tenant config: {}", e);
                        eprintln!("   Continuing in single-tenant mode...");
                        None
                    }
                }
            }
            None => None,
        };

        if tenant_map.is_none() {
            if auth_key.is_some() {
                println!("Auth enabled (pmb-v1 key required)");
            }
            if let Some(ref dir) = data_dir {
                println!("Tenant data directory: {}", dir);
            }
        }

        let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).unwrap();
        println!("SQ v{} listening on port {} (max {} concurrent connections)...",
            env!("CARGO_PKG_VERSION"), port, MAX_CONCURRENT_CONNECTIONS);

        let state = Arc::new(Mutex::new(ServerState {
            loaded_phext: String::new(),
            loaded_map: Default::default(),
        }));

        let mut connection_id: u64 = 0;
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    // --- Guard: reject when at capacity ---
                    let current = ACTIVE_CONNECTIONS.load(Ordering::Relaxed);
                    if current >= MAX_CONCURRENT_CONNECTIONS {
                        eprintln!("[!] Connection limit reached ({}/{}), rejecting",
                            current, MAX_CONCURRENT_CONNECTIONS);
                        let mut s = stream;
                        send_response(&mut s, 503, "Service Unavailable: connection limit reached");
                        continue;
                    }

                    // --- Set timeouts to prevent idle threads from piling up ---
                    if let Err(e) = stream.set_read_timeout(Some(Duration::from_secs(READ_TIMEOUT_SECS))) {
                        eprintln!("[!] Failed to set read timeout: {}", e);
                    }
                    if let Err(e) = stream.set_write_timeout(Some(Duration::from_secs(WRITE_TIMEOUT_SECS))) {
                        eprintln!("[!] Failed to set write timeout: {}", e);
                    }

                    connection_id += 1;
                    ACTIVE_CONNECTIONS.fetch_add(1, Ordering::Relaxed);
                    let state = Arc::clone(&state);
                    let auth_key = auth_key.clone();
                    let data_dir = data_dir.clone();
                    let tenants = tenant_map.clone();
                    let cid = connection_id;
                    std::thread::spawn(move || {
                        handle_tcp_connection(state, cid, stream, &auth_key, &data_dir, &tenants);
                        ACTIVE_CONNECTIONS.fetch_sub(1, Ordering::Relaxed);
                    });
                }
                Err(e) => {
                    eprintln!("Accept error: {}", e);
                    continue;
                }
            }
        }
        return Ok(());
    }

    // -----------------------------------------------------------------------
    // Daemon mode: shared memory IPC
    // -----------------------------------------------------------------------
    if is_basic_or_share(command.clone()) {
        recreate_sq_work_files();
    }

    let (shmem, wkmem) = loop {
        let shmem: Shmem = match create_shared_segment() {
            Ok(s) => { s }
            Err(ShmemError::LinkExists) => { ShmemConf::new().flink(SHARED_NAME).open()? }
            Err(e) => { return Err(Box::new(e)); }
        };
        let wkmem: Shmem = match create_work_segment() {
            Ok(w) => { w }
            Err(ShmemError::LinkExists) => { ShmemConf::new().flink(WORK_NAME).open()? }
            Err(e) => { return Err(Box::new(e)); }
        };

        break (shmem, wkmem);
    };

    if shmem.is_owner() && is_basic_or_share(command) { return server(shmem, wkmem); }
    else { return client(shmem, wkmem); }
}

// -----------------------------------------------------------------------------------------------------------
// fetches an incoming scroll from shared memory
// -----------------------------------------------------------------------------------------------------------
fn fetch_message(shmem: *mut u8, start: usize) -> String {
    let length_size = 20;
    unsafe {
        let raw = std::slice::from_raw_parts(shmem.add(start), length_size);
        let length_string = String::from_utf8_unchecked(raw.to_vec()).to_string();
        let length: usize = length_string.parse().unwrap_or(0);
        if length == 0 {
            return String::new();
        }
        let unparsed = std::slice::from_raw_parts(shmem.add(start+length_size), length);
        return String::from_utf8_unchecked(unparsed.to_vec()).to_string();
    }
}

// -----------------------------------------------------------------------------------------------------------
// sends a scroll over shared memory
// -----------------------------------------------------------------------------------------------------------
fn send_message(shmem: *mut u8, start: usize, encoded: String) {
    let prepared = format!("{:020}{}", encoded.len(), encoded);
    unsafe {
        // Zero only the bytes we need, not the entire 1 GB segment
        let zero_length = prepared.len() + 1;
        std::ptr::write_bytes(shmem.add(start), 0, zero_length);
        std::ptr::copy_nonoverlapping(prepared.as_ptr(), shmem.add(start), prepared.len());
    }
}

// -----------------------------------------------------------------------------------------------------------
// uses percent encoding to fetch URL parameters
// -----------------------------------------------------------------------------------------------------------
fn url_decode(encoded: &str) -> String {
    let stage1 = encoded.to_string().replace("+", " ");
    let stage2 = percent_encoding::percent_decode(stage1.as_bytes())
        .decode_utf8_lossy()
        .to_string();

    return stage2;
}

// -----------------------------------------------------------------------------------------------------------
// quick key/value parsing from a query string
// -----------------------------------------------------------------------------------------------------------
fn parse_query_string(query: &str) -> HashMap<String, String> {
    let mut result = HashMap::new();

    for pair in query.split('&') {
        let mut key_value = pair.splitn(2, '=');
        if let (Some(key), Some(value)) = (key_value.next(), key_value.next()) {
            result.insert(
                url_decode(key),
                url_decode(value),
            );
        }
    }

    return result;
}

// -----------------------------------------------------------------------------------------------------------
// minimal HTTP parsing
// -----------------------------------------------------------------------------------------------------------
fn request_parse(request: &HttpRequest) -> Option<HashMap<String, String>> {
    let mut result = HashMap::new();
    let content = String::from_utf8_lossy(&request.content).to_string();
    let lines = request.header.split("\r\n");
    for line in lines {
        let mut parts = line.splitn(2, '?');
        if let (Some(key), Some(value)) = (parts.next(), parts.next()) {
            if key.contains("favicon.ico") { return None; }
            result = parse_query_string(value.strip_suffix(" HTTP/1.1").unwrap_or(value));
        }
        break;
    }
    if content.len() > 0 {
        result.insert("content".to_string(), content);
    }

    return Some(result);
}

pub struct HttpRequest {
    pub header: String,
    pub content: Vec<u8>,
}

pub fn read_http_request(stream: &mut TcpStream) -> std::io::Result<HttpRequest> {
    let mut buffer = Vec::new();
    let mut temp = [0u8; 1024];
    let header_end;
    loop {
        let n = stream.read(&mut temp)?;
        if n == 0 {
            return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "connection closed"));
        }
        buffer.extend_from_slice(&temp[..n]);

        if let Some(pos) = buffer.windows(4).position(|w| w == b"\r\n\r\n") {
            header_end = pos + 4;
            break;
        }

        // Guard: reject absurdly large headers
        if buffer.len() > ABSURD_HEADER_SIZE {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "header too large"));
        }
    }

    let header_bytes = &buffer[..header_end];
    let header_str = String::from_utf8_lossy(header_bytes).to_string();

    let content_length = header_str
        .lines()
        .find(|line| line.to_ascii_lowercase().starts_with("content-length:"))
        .and_then(|line| line.split(':').nth(1))
        .and_then(|val| val.trim().parse::<usize>().ok())
        .unwrap_or(0);

    // Guard: reject bodies larger than MAX_BODY_SIZE
    if content_length > MAX_BODY_SIZE {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("body too large: {} bytes (max {})", content_length, MAX_BODY_SIZE),
        ));
    }

    let mut content = buffer[header_end..].to_vec();
    while content.len() < content_length {
        let remaining = content_length - content.len();
        let mut chunk = vec![0u8; remaining.min(1024)];
        let n = stream.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        content.extend_from_slice(&chunk[..n]);
    }

    Ok(HttpRequest {
        header: header_str,
        content,
    })
}

// -----------------------------------------------------------------------------------------------------------
// TCP connection handler — catches panics so the server never dies from a bad request
// -----------------------------------------------------------------------------------------------------------
fn handle_tcp_connection(
    state: Arc<Mutex<ServerState>>,
    connection_id: u64,
    mut stream: TcpStream,
    auth_key: &Option<String>,
    data_dir: &Option<String>,
    tenant_map: &Option<Arc<HashMap<String, config::TenantConfig>>>,
) {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        handle_tcp_connection_inner(state, connection_id, &mut stream, auth_key, data_dir, tenant_map)
    }));
    if let Err(e) = result {
        eprintln!("[#{}] panic: {:?}", connection_id, e);
        send_response(&mut stream, 500, "Internal Server Error");
    }
}

// -----------------------------------------------------------------------------------------------------------
// Inner connection handler — all the actual HTTP logic
// -----------------------------------------------------------------------------------------------------------
fn handle_tcp_connection_inner(
    state: Arc<Mutex<ServerState>>,
    connection_id: u64,
    stream: &mut TcpStream,
    auth_key: &Option<String>,
    data_dir: &Option<String>,
    tenant_map: &Option<Arc<HashMap<String, config::TenantConfig>>>,
) {
    // Phase 1: Read request (no lock needed)
    let http_request = match read_http_request(stream) {
        Ok(req) => req,
        Err(e) => {
            let kind = e.kind();
            // Distinguish between client misbehavior and normal timeouts
            if kind == std::io::ErrorKind::InvalidData {
                eprintln!("[#{}] rejected: {}", connection_id, e);
                send_response(stream, 413, &format!("{}", e));
            } else {
                eprintln!("[#{}] read error: {}", connection_id, e);
            }
            return;
        }
    };
    let request = &http_request.header;

    // Handle CORS preflight
    if request.starts_with("OPTIONS ") {
        let _ = stream.write_all(
            b"HTTP/1.1 204 No Content\r\n\
              Access-Control-Allow-Origin: *\r\n\
              Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\
              Access-Control-Allow-Headers: Authorization, Content-Type\r\n\
              Access-Control-Max-Age: 86400\r\n\r\n"
        );
        return;
    }

    if !request.starts_with("GET ") && !request.starts_with("POST ") {
        send_response(stream, 400, "Bad Request");
        return;
    }

    // Multi-tenant auth: resolve token → tenant data_dir, or fall back to single-key mode
    let resolved_data_dir: Option<String>;
    if let Some(ref tenants) = tenant_map {
        // Multi-tenant mode: extract token and look up tenant
        let token = extract_header(request, "authorization")
            .map(|a| {
                let a = a.trim().to_string();
                if a.to_lowercase().starts_with("bearer ") { a[7..].trim().to_string() } else { a }
            });
        match token {
            Some(ref t) if tenants.contains_key(t) => {
                resolved_data_dir = Some(tenants[t].data_dir.clone());
            }
            _ => {
                send_response(stream, 401, "Unauthorized");
                return;
            }
        }
    } else {
        // Single-tenant mode: use --key / --data-dir
        if !validate_auth(request, auth_key) {
            send_response(stream, 401, "Unauthorized");
            return;
        }
        resolved_data_dir = data_dir.clone();
    }

    // Phase 2: Parse request (no lock needed)
    let parsed = match request_parse(&http_request) {
        None => return, // favicon.ico etc
        Some(x) => x,
    };

    let nothing = String::new();
    let scroll_param = parsed.get("s").unwrap_or(&nothing).clone();
    let coord = parsed.get("c").unwrap_or(&nothing).clone();
    let phext_name = parsed.get("p").unwrap_or(&nothing).clone();

    let phext = match validate_tenant_path(&phext_name, &resolved_data_dir) {
        Some(path) => path,
        None => {
            send_response(stream, 403, "Forbidden: invalid phext path");
            return;
        }
    };

    let algo_str = parsed.get("algo").unwrap_or(&nothing).clone();
    let limit_str = parsed.get("limit").unwrap_or(&nothing).clone();
    let algorithm = if algo_str == "checksum" { HashAlgorithm::Checksum } else { HashAlgorithm::Xor };
    let limit: usize = limit_str.parse().unwrap_or(100);

    // Route matching
    let command:String;
    let mut scroll = scroll_param.clone();
    let mut reload_needed = false; // determined under lock

    if request.starts_with("GET /api/v2/load") {
        command = "load".to_string();
        reload_needed = true;
    } else if request.starts_with("GET /api/v2/select") {
        command = "select".to_string();
    } else if request.starts_with("GET /api/v2/insert") {
        command = "insert".to_string();
    } else if request.starts_with("POST /api/v2/insert") {
        command = "insert".to_string();
        if let Some(content) = parsed.get("content") { scroll = content.clone(); }
    } else if request.starts_with("GET /api/v2/update") {
        command = "update".to_string();
    } else if request.starts_with("POST /api/v2/update") {
        command = "update".to_string();
        if let Some(content) = parsed.get("content") { scroll = content.clone(); }
    } else if request.starts_with("POST /api/v2/where") {
        command = "where".to_string();
        if let Some(content) = parsed.get("content") { scroll = content.clone(); }
    } else if request.starts_with("GET /api/v2/delete") {
        command = "delete".to_string();
    } else if request.starts_with("GET /api/v2/status") {
        command = "status".to_string();
    } else if request.starts_with("GET /api/v2/checksum") {
        command = "checksum".to_string();
    } else if request.starts_with("GET /api/v2/toc") {
        command = "toc".to_string();
    } else if request.starts_with("GET /api/v2/get") {
        command = "get".to_string();
    } else if request.starts_with("GET /api/v2/delta") {
        command = "delta".to_string();
    } else if request.starts_with("POST /api/v2/delta") {
        command = "delta".to_string();
        if let Some(content) = parsed.get("content") { scroll = content.clone(); }
    } else if request.starts_with("GET /api/v2/version") {
        command = "version".to_string();
    } else if request.starts_with("GET /api/v2/json-export") {
        command = "json-export".to_string();
        reload_needed = true;
    } else {
        send_response(stream, 404, "Not Found");
        return;
    }

    // Phase 3: Acquire lock, process, optionally write to disk
    let output = {
        let mut state = state.lock().unwrap_or_else(|poisoned| {
            eprintln!("[#{}] recovering from poisoned mutex", connection_id);
            poisoned.into_inner()
        });

        // Check if we need to reload (phext changed or explicit load)
        if reload_needed || state.loaded_phext != phext {
            state.loaded_map = fetch_source(phext.clone());
            state.loaded_phext = phext.clone();
        }

        let mut output = String::new();
        let _ = sq::process(
            connection_id, phext.clone(), &mut output, command.clone(),
            &mut state.loaded_map, phext::to_coordinate(coord.as_str()),
            scroll.clone(), phext.clone(), algorithm, limit,
        );

        // Only flush to disk when the command actually changed something
        // Uses implode_ref: borrows the map instead of cloning it
        if is_mutation(&command) {
            let phext_buffer = sq::implode_ref(&state.loaded_map);
            if let Err(e) = std::fs::write(&phext, &phext_buffer) {
                eprintln!("[#{}] disk write failed for {}: {}", connection_id, phext, e);
            }
        }

        output
        // lock released here
    };

    // Phase 4: Send response (no lock needed)
    let length = output.len();
    let response = format!(
        "HTTP/1.1 200 OK\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {length}\r\n\r\n{output}"
    );
    let _ = stream.write_all(response.as_bytes());
}

// -----------------------------------------------------------------------------------------------------------
// keeps track of the event signaling overhead (4 bytes)
// -----------------------------------------------------------------------------------------------------------
fn event_byte_offset(offset: usize) -> usize {
    offset + 4
}

// -----------------------------------------------------------------------------------------------------------
// daemon mode server processing loop
// -----------------------------------------------------------------------------------------------------------
fn server(shmem: Shmem, wkmem: Shmem) -> Result<(), Box<dyn std::error::Error>> {
    let mut connection_id: u64 = 0;

    let (evt, evt_used_bytes) = unsafe { Event::new(shmem.as_ptr(), true)? };
    let (work, _used_work_bytes) = unsafe { Event::new(wkmem.as_ptr(), true)? };

    let length_offset  = event_byte_offset(evt_used_bytes);

    let ps1: phext::Coordinate = phext::to_coordinate("1.1.1/1.1.1/1.1.1");
    let ps2: phext::Coordinate = phext::to_coordinate("1.1.1/1.1.1/1.1.2");
    let ps3: phext::Coordinate = phext::to_coordinate("1.1.1/1.1.1/1.1.3");

    let command = env::args().nth(1).unwrap_or("".to_string());
    let mut filename: String;
    if command == "basic" {
        filename = "index".to_string();
    } else {
        filename = env::args().nth(2).expect("Usage: sq share <phext> or sq host <port>");
    }

    println!("Operating in daemon mode.");

    if filename == "init" {
        println!("Init was previously completed - launching with world.phext...");
        filename = "world.phext".to_string();
    }
    println!("SQ v{}", env!("CARGO_PKG_VERSION"));
    println!("Loading {} into memory...", filename);

    let mut phext_buffer = fetch_source(filename.clone());
    println!("Serving {} scrolls.", phext_buffer.len());

    loop {
        evt.wait(Timeout::Infinite)?;
        connection_id += 1;

        let parts = fetch_message(shmem.as_ptr(), length_offset);

        let command = phext::fetch(parts.as_str(), ps1);
        let argtemp = phext::fetch(parts.as_str(), ps2);
        let coordinate = phext::to_coordinate(argtemp.as_str());
        let update = phext::fetch(parts.as_str(), ps3);

        let mut scroll = String::new();
        let done = sq::process(connection_id, filename.clone(), &mut scroll, command, &mut phext_buffer, coordinate, update, argtemp.clone(), HashAlgorithm::Xor, 100);
        let scroll_length = scroll.len();

        send_message(shmem.as_ptr(), length_offset, scroll);
        work.set(EventState::Signaled)?;
        let scroll_count = phext_buffer.len();
        println!("[#{}] {} bytes ({} contains {} scrolls)", connection_id, scroll_length, filename, scroll_count);

        if done {
            println!("Returning to the shell...");
            break;
        }
    }

    println!("SQ Shutdown Complete.");
    Ok(())
}

// -----------------------------------------------------------------------------------------------------------
// short-circuit media file types
// -----------------------------------------------------------------------------------------------------------
fn is_media_resource(filename: &str) -> bool {
    filename.ends_with(".jpg") ||
    filename.ends_with(".mp4") ||
    filename.ends_with(".mp3") ||
    filename.ends_with(".gif") ||
    filename.ends_with(".webp") ||
    filename.ends_with(".png")
}

// -----------------------------------------------------------------------------------------------------------
// daemon mode client processing loop
// -----------------------------------------------------------------------------------------------------------
fn client(shmem: Shmem, wkmem: Shmem) -> Result<(), Box<dyn std::error::Error>> {

    let (evt, evt_used_bytes) = unsafe { Event::from_existing(shmem.as_ptr())? };
    let (work, _work_used_bytes) = unsafe { Event::from_existing(wkmem.as_ptr())? };
    let length_offset  = event_byte_offset(evt_used_bytes);

    let nothing: String = String::new();
    let args: Vec<String> = env::args().collect();

    let command = args.get(1).unwrap_or(&nothing);
    let usage = "Usage: sq <command> <coordinate> <message>";

    if args.len() < sq::args_required(command) {
        if command == "init" {
            if std::path::Path::new(SHARED_NAME).exists() {
                _ = std::fs::remove_file(SHARED_NAME);
            }
            if std::path::Path::new(WORK_NAME).exists() {
                _ = std::fs::remove_file(WORK_NAME);
            }
            println!("Cleared working files");
            return Ok(());
        }
        println!("{}", usage);
        return Ok(());
    }

    let mut coordinate = args.get(2).unwrap_or(&nothing).to_string();
    let mut message: String = args.get(3).unwrap_or(&nothing).to_string();
    if command == "push" {
        message = phext::implode(fetch_source(message));
    }
    if command == "slurp" {
        let mut summary = String::new();
        let dir = Path::new(&message);
        println!("Slurping {message}...");
        let mut coord = phext::to_coordinate(coordinate.as_str());
        let toc = coord;
        for entry in std::fs::read_dir(dir).ok().into_iter().flat_map(|e| e) {
            if let Ok(entry) = entry {
                coord.scroll_break();
                if coord.x.scroll == (phext::COORDINATE_MAXIMUM - 1) {
                    coord.section_break();
                }
                if coord.x.section == (phext::COORDINATE_MAXIMUM - 1) {
                    coord.chapter_break();
                }
                if coord.x.chapter == (phext::COORDINATE_MAXIMUM - 1) {
                    coord.book_break();
                    println!("Warning: Slurp exceeded 900M scrolls.");
                }
                let path = entry.path();
                let mut filename = String::new();
                if let Some(parsed_filename) = path.file_name() {
                    filename = parsed_filename.to_string_lossy().to_string();
                }
                let checker = filename.to_lowercase();
                if is_media_resource(checker.as_str()) {
                    summary.push_str(&format!("{coord} {filename} (Resource)\n").as_str());
                    continue;
                }
                if path.is_file() {
                    if let Ok(content) = fs::read_to_string(&path) {
                        summary.push_str(&format!("{coord} {filename}\n").as_str());
                        client_submit(command, coordinate.as_str(), content.as_str(), shmem.as_ptr(), length_offset);
                        coordinate = coord.to_string();
                        evt.set(EventState::Signaled)?;
                        work.wait(Timeout::Infinite)?;
                        client_response(shmem.as_ptr(), length_offset, command, message.as_str(), coordinate.as_str());
                    }
                }
            }
        }
        coordinate = toc.to_string();
        message = summary;
    }

    client_submit(command, coordinate.as_str(), message.as_str(), shmem.as_ptr(), length_offset);
    evt.set(EventState::Signaled)?;
    work.wait(Timeout::Infinite)?;
    client_response(shmem.as_ptr(), length_offset, command, message.as_str(), coordinate.as_str());

    Ok(())
}

// -----------------------------------------------------------------------------------------------------------
// daemon mode client submission process using a simple phext structure
// -----------------------------------------------------------------------------------------------------------
fn client_submit(command: &str, coordinate: &str, message: &str, shmem: *mut u8, length_offset: usize)
{
    let mut encoded = String::new();
    encoded.push_str(command);
    encoded.push(phext::SCROLL_BREAK);
    encoded.push_str(coordinate);
    encoded.push(phext::SCROLL_BREAK);
    encoded.push_str(message);
    encoded.push(phext::SCROLL_BREAK);

    send_message(shmem, length_offset, encoded);
}

// -----------------------------------------------------------------------------------------------------------
// show progress on daemon mode requests
// -----------------------------------------------------------------------------------------------------------
fn client_response(shmem: *mut u8, length_offset: usize, command: &str, message: &str, coordinate: &str)
{
    let mut response = fetch_message(shmem, length_offset);
    if command == "pull" {
        let filename = message;
        let _ = std::fs::write(filename, response.clone());
        response = format!("Exported scroll at {coordinate} to {filename}.").to_string();
    }
    if coordinate.len() > 0 {
        println!("{coordinate}: {response}");
    } else {
        println!("{response}");
    }
}

// -----------------------------------------------------------------------------------------------------------
// Multi-tenant REST API server (SQ v0.5.5)
// Loads tenant config and serves requests from single process
// -----------------------------------------------------------------------------------------------------------
fn run_multi_tenant_server(port: &str, config_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Load tenant configuration
    let tenant_config = config::load_config(config_path)?;
    println!("SQ v{} - Multi-tenant mode", env!("CARGO_PKG_VERSION"));
    println!("Loaded {} tenants from {}", tenant_config.tenants.len(), config_path);
    
    // Ensure all tenant data directories exist
    for (_token, tenant) in &tenant_config.tenants {
        let _ = std::fs::create_dir_all(&tenant.data_dir);
        println!("  - {} ({})", tenant.name, tenant.data_dir);
    }
    
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port))?;
    println!("Listening on port {}...", port);
    
    let active_connections = Arc::new(AtomicUsize::new(0));
    let tenant_config = Arc::new(tenant_config);
    
    for stream in listener.incoming() {
        let stream = match stream {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to accept connection: {}", e);
                continue;
            }
        };
        
        let count = active_connections.fetch_add(1, Ordering::SeqCst) + 1;
        if count > MAX_CONCURRENT_CONNECTIONS {
            active_connections.fetch_sub(1, Ordering::SeqCst);
            eprintln!("Rejecting connection (at capacity: {} concurrent)", MAX_CONCURRENT_CONNECTIONS);
            let _ = stream.shutdown(std::net::Shutdown::Both);
            continue;
        }
        
        let tenant_config = Arc::clone(&tenant_config);
        let active_connections = Arc::clone(&active_connections);
        
        std::thread::spawn(move || {
            handle_multi_tenant_connection(stream, &tenant_config);
            active_connections.fetch_sub(1, Ordering::SeqCst);
        });
    }
    
    Ok(())
}

// -----------------------------------------------------------------------------------------------------------
// Multi-tenant connection handler
// -----------------------------------------------------------------------------------------------------------
fn handle_multi_tenant_connection(mut stream: TcpStream, config: &config::ServerConfig) {
    // Set timeouts
    let timeout = Duration::from_secs(30);
    let _ = stream.set_read_timeout(Some(timeout));
    let _ = stream.set_write_timeout(Some(timeout));
    
    let http_request = match read_http_request(&mut stream) {
        Ok(req) => req,
        Err(e) => {
            eprintln!("Failed to read request: {}", e);
            send_response(&mut stream, 400, "Bad Request");
            return;
        }
    };
    
    let request = &http_request.header;
    
    // Handle OPTIONS (CORS preflight) without auth
    if request.starts_with("OPTIONS ") {
        send_response(&mut stream, 204, "");
        return;
    }
    
    // Validate HTTP method
    if !request.starts_with("GET ") && !request.starts_with("POST") {
        send_response(&mut stream, 400, "Bad Request");
        return;
    }
    
    // Multi-tenant auth: extract token and lookup tenant
    let tenant = match extract_auth_token_multi(request, config) {
        Some(t) => t,
        None => {
            send_response(&mut stream, 401, "Unauthorized");
            return;
        }
    };
    
    // Parse request
    let parsed = match request_parse(&http_request) {
        Some(p) => p,
        None => {
            send_response(&mut stream, 400, "Bad Request");
            return;
        }
    };
    
    let nothing = String::new();
    let mut scroll = parsed.get("s").unwrap_or(&nothing);
    let coord = parsed.get("c").unwrap_or(&nothing);
    let phext_name = parsed.get("p").unwrap_or(&nothing);
    
    // Validate tenant path
    let phext_path = match validate_tenant_path_multi(phext_name, &tenant.data_dir) {
        Some(path) => path,
        None => {
            send_response(&mut stream, 403, "Forbidden: Invalid phext path");
            return;
        }
    };
    
    // Parse algorithm and limit
    let algo_str = parsed.get("algo").unwrap_or(&nothing);
    let limit_str = parsed.get("limit").unwrap_or(&nothing);
    let algorithm = if algo_str == "checksum" { HashAlgorithm::Checksum } else { HashAlgorithm::Xor };
    let limit: usize = limit_str.parse().unwrap_or(100);
    
    // Determine command and reload flag
    let mut command = String::new();
    
    if request.starts_with("GET /api/v2/load") {
        command = "load".to_string();
    } else if request.starts_with("GET /api/v2/select") {
        command = "select".to_string();
    } else if request.starts_with("GET /api/v2/insert") {
        command = "insert".to_string();
    } else if request.starts_with("POST /api/v2/insert") {
        command = "insert".to_string();
        if parsed.contains_key("content") {
            scroll = &parsed["content"];
        }
    } else if request.starts_with("GET /api/v2/update") {
        command = "update".to_string();
    } else if request.starts_with("POST /api/v2/update") {
        command = "update".to_string();
        if parsed.contains_key("content") {
            scroll = &parsed["content"];
        }
    } else if request.starts_with("POST /api/v2/where") {
        command = "where".to_string();
        if parsed.contains_key("content") {
            scroll = &parsed["content"];
        }
    } else if request.starts_with("GET /api/v2/delete") {
        command = "delete".to_string();
    } else if request.starts_with("GET /api/v2/status") {
        command = "status".to_string();
    } else if request.starts_with("GET /api/v2/checksum") {
        command = "checksum".to_string();
    } else if request.starts_with("GET /api/v2/toc") {
        command = "toc".to_string();
    } else if request.starts_with("GET /api/v2/get") {
        command = "get".to_string();
    } else if request.starts_with("GET /api/v2/delta") {
        command = "delta".to_string();
    } else if request.starts_with("POST /api/v2/delta") {
        command = "delta".to_string();
        if parsed.contains_key("content") {
            scroll = &parsed["content"];
        }
    } else if request.starts_with("GET /api/v2/version") {
        command = "version".to_string();
    } else if request.starts_with("GET /api/v2/json-export") {
        command = "json-export".to_string();
    } else {
        send_response(&mut stream, 404, "Not Found");
        return;
    }
    
    // Ensure tenant data directory exists before loading phext
    if let Some(parent) = std::path::Path::new(&phext_path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    
    // Load phext source
    let mut loaded_map = fetch_source(phext_path.clone());
    
    // Process command
    let mut output = String::new();
    let _ = sq::process(
        0,
        phext_path.clone(),
        &mut output,
        command.clone(),
        &mut loaded_map,
        phext::to_coordinate(coord.as_str()),
        scroll.clone(),
        phext_path.clone(),
        algorithm,
        limit
    );
    
    // Save if mutation
    if is_mutation(&command) {
        let phext_buffer = phext::implode(loaded_map);
        let _ = std::fs::write(&phext_path, phext_buffer);
    }
    
    // Send response
    send_response(&mut stream, 200, &output);
}

// -----------------------------------------------------------------------------------------------------------
// Extract auth token and lookup tenant config
// Supports both Authorization: Bearer <token> and X-SQ-API-Key: <token>
// -----------------------------------------------------------------------------------------------------------
fn extract_auth_token_multi<'a>(header: &str, config: &'a config::ServerConfig) -> Option<&'a config::TenantConfig> {
    for line in header.lines() {
        let lower_line = line.to_lowercase();
        
        // Try Authorization header first
        if lower_line.starts_with("authorization:") {
            if let Some(value) = line.split(':').nth(1) {
                let token = value.trim();
                // Strip "Bearer " prefix if present
                let token = if token.to_lowercase().starts_with("bearer ") {
                    &token[7..].trim()
                } else {
                    token
                };
                // Lookup tenant by token
                if let Some(tenant) = config.tenants.get(token) {
                    return Some(tenant);
                }
            }
        }
        
        // Fallback to X-SQ-API-Key (backward compatibility with Phext Notepad)
        if lower_line.starts_with("x-sq-api-key:") {
            if let Some(value) = line.split(':').nth(1) {
                let token = value.trim();
                // Lookup tenant by token
                if let Some(tenant) = config.tenants.get(token) {
                    return Some(tenant);
                }
            }
        }
    }
    None
}

// -----------------------------------------------------------------------------------------------------------
// Validate phext path for multi-tenant mode
// -----------------------------------------------------------------------------------------------------------
fn validate_tenant_path_multi(phext_name: &str, data_dir: &str) -> Option<String> {
    // Reject any path traversal attempts
    if phext_name.contains("..") || phext_name.contains('/') || phext_name.contains('\\') {
        return None;
    }
    Some(format!("{}/{}.phext", data_dir, phext_name))
}

// -----------------------------------------------------------------------------------------------------------
// provides a way to infer a phext coordinate from input text
// -----------------------------------------------------------------------------------------------------------
fn infer_coordinate(text: &str, limit: usize, algorithm: HashAlgorithm) -> phext::Coordinate
{
    match algorithm {
        HashAlgorithm::Xor => xor_phoken_hash(text, limit),
        HashAlgorithm::Checksum => checksum_to_coordinate(text),
    }
}

// -----------------------------------------------------------------------------------------------------------
// XOR-based coordinate inference from phokens
// -----------------------------------------------------------------------------------------------------------
fn xor_phoken_hash(text: &str, limit: usize) -> phext::Coordinate
{
   let phokens = phext::phokenize(text);

   let mut composite = phext::Coordinate::default();
   for phoken in phokens {
      if phoken.scroll.len() >= limit {
         composite.x.scroll ^= phoken.coord.x.scroll;
         composite.x.section ^= phoken.coord.x.section;
         composite.x.chapter ^= phoken.coord.x.chapter;
         composite.y.book ^= phoken.coord.y.book;
         composite.y.volume ^= phoken.coord.y.volume;
         composite.y.collection ^= phoken.coord.y.collection;
         composite.z.series ^= phoken.coord.z.series;
         composite.z.shelf ^= phoken.coord.z.shelf;
         composite.z.library ^= phoken.coord.z.library;
      }
   }

   return composite;
}

// -----------------------------------------------------------------------------------------------------------
// Checksum-based coordinate inference - maps hash bytes to coordinate components
// -----------------------------------------------------------------------------------------------------------
fn checksum_to_coordinate(text: &str) -> phext::Coordinate
{
    let hash = phext::checksum(text);
    let bytes: Vec<u8> = hash.bytes().collect();

    let get_component = |start: usize| -> usize {
        if start + 1 < bytes.len() {
            let val = (((bytes[start] as usize) << 8) | (bytes[start + 1] as usize)) % 999;
            if val == 0 { 1 } else { val }
        } else if start < bytes.len() {
            let val = (bytes[start] as usize) % 999;
            if val == 0 { 1 } else { val }
        } else {
            1
        }
    };

    phext::Coordinate {
        z: phext::ZCoordinate {
            library: get_component(0),
            shelf: get_component(2),
            series: get_component(4),
        },
        y: phext::YCoordinate {
            collection: get_component(6),
            volume: get_component(8),
            book: get_component(10),
        },
        x: phext::XCoordinate {
            chapter: get_component(12),
            section: get_component(14),
            scroll: get_component(16),
        },
    }
}
