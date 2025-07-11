//------------------------------------------------------------------------------------------------------------
// file: main.rs
// purpose: provides primary program logic for sq - determining daemon mode vs listening mode
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

mod sq;
mod tests;

const SHARED_SEGMENT_SIZE: usize = 1024*1024*1024; // 1 GB limit
const MAX_BUFFER_SIZE: usize = SHARED_SEGMENT_SIZE/2;
const WORK_SEGMENT_SIZE: usize = 1024;

const SHARED_NAME: &str = ".sq/link";
const WORK_NAME: &str = ".sq/work";

// -----------------------------------------------------------------------------------------------------------
// Loads + explodes a source phext from disk into memory
// -----------------------------------------------------------------------------------------------------------
fn fetch_source(filename: String) -> HashMap::<phext::Coordinate, String> {
    let message = format!("Unable to open {}", filename);
    let exists = std::fs::exists(filename.clone()).unwrap_or(false);
    if exists == false {
        let _ = std::fs::write(filename.clone(), "");
    }
    let mut buffer:String = std::fs::read_to_string(filename).expect(&message);

    if buffer.len() > MAX_BUFFER_SIZE {
        buffer = buffer[0..MAX_BUFFER_SIZE].to_string();
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
// sq program loop
// -----------------------------------------------------------------------------------------------------------
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sq_exists = std::fs::exists(".sq").unwrap_or(false);
    if sq_exists == false {
        let _ = std::fs::create_dir(".sq");
    }

    let command = env::args().nth(1).unwrap_or("".to_string());
    let phext_or_port = env::args().nth(2).unwrap_or("".to_string());
    let exists = std::fs::exists(phext_or_port.clone()).unwrap_or(false);
    let is_port_number = phext_or_port.parse::<u16>().is_ok();

    let mut loaded_phext = String::new();
    let mut loaded_map: HashMap<phext::Coordinate, String> = Default::default();

    if command == "host" && exists == false && phext_or_port.len() > 0 && is_port_number {
        let port = phext_or_port;
        let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).unwrap();
        println!("Listening on port {port}...");

        let mut connection_id: u64 = 0;
        for stream in listener.incoming() {
            let stream = stream.unwrap();
            connection_id += 1;
            handle_tcp_connection(&mut loaded_phext, &mut loaded_map, connection_id, stream);
        }
        return Ok(());
    }

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
    let zeros = vec![0 as u8; SHARED_SEGMENT_SIZE];
    let prepared = format!("{:020}{}", encoded.len(), encoded);
    unsafe {
        let zero_length = prepared.len() + 1;
        std::ptr::copy_nonoverlapping(zeros.as_ptr(), shmem.add(start), zero_length);
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
fn request_parse(request: &HttpRequest) -> HashMap<String, String> {
    let mut result = HashMap::new();
    let content = String::from_utf8_lossy(&request.content).to_string();
    let lines = request.header.split("\r\n");
    for line in lines {
        let mut parts = line.splitn(2, '?');
        if let (Some(_key), Some(value)) = (parts.next(), parts.next()) {
            result = parse_query_string(value.strip_suffix(" HTTP/1.1").unwrap_or(value));
        }
        break; // ignore the rest of the headers
    }
    if content.len() > 0 {
        result.insert("content".to_string(), content);
    }

    return result;
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
    }

    // Split header and content
    let header_bytes = &buffer[..header_end];
    let header_str = String::from_utf8_lossy(header_bytes).to_string();

    // Check Content-Length
    let content_length = header_str
        .lines()
        .find(|line| line.to_ascii_lowercase().starts_with("content-length:"))
        .and_then(|line| line.split(':').nth(1))
        .and_then(|val| val.trim().parse::<usize>().ok())
        .unwrap_or(0);

    // Read remaining content (if any)
    let mut content = buffer[header_end..].to_vec(); // Already-read part of content
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
// minimal TCP socket handling
// -----------------------------------------------------------------------------------------------------------
fn handle_tcp_connection(loaded_phext: &mut String, loaded_map: &mut HashMap<phext::Coordinate, String>, connection_id: u64, mut stream: std::net::TcpStream) {
    let http_request: HttpRequest = read_http_request(&mut stream).expect("unexpected socket failure");
    let request = &http_request.header;
    if request.starts_with("GET ") == false &&
       request.starts_with("POST") == false {
        stream.write_all("HTTP/1.1 400 Bad Request\r\n".as_bytes()).unwrap();
        println!("Ignoring {}", request);
        return;
    }
    println!("Request: {}", request);

    let headers = "HTTP/1.1 200 OK";
    let parsed = request_parse(&http_request);
    let nothing = String::new();
    let mut scroll = parsed.get("s").unwrap_or(&nothing);
    let coord  = parsed.get("c").unwrap_or(&nothing);
    let phext  = parsed.get("p").unwrap_or(&nothing).to_owned() + ".phext";
    let mut reload_needed = *loaded_phext != phext;
    let mut output = String::new();
    let mut command = String::new();
    if request.starts_with("GET /api/v2/load") {
        command = "load".to_string();
        *loaded_map = fetch_source(phext.clone());
        *loaded_phext = phext.clone();
        reload_needed = false;
    } else if request.starts_with("GET /api/v2/select") {
        command = "select".to_string();
    } else if request.starts_with("GET /api/v2/insert") {
        command = "insert".to_string();
    } else if request.starts_with("POST /api/v2/insert") {
        command = "insert".to_string();
        if parsed.contains_key("content") {
            scroll = &parsed["content"];
        } else { scroll = &nothing; }
    } else if request.starts_with("GET /api/v2/update") {
        command = "update".to_string();
    } else if request.starts_with("POST /api/v2/update") {
        command = "update".to_string();
        if parsed.contains_key("content") {
            scroll = &parsed["content"];
        } else { scroll = &nothing; }
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
        } else { scroll = &nothing; }
    } else if request.starts_with("GET /api/v2/version") {
        command = "version".to_string();
    }

    if reload_needed {
        *loaded_map = fetch_source(phext.clone());
        *loaded_phext = phext.clone();
    }

    let _ = sq::process(connection_id, phext.clone(), &mut output, command, &mut *loaded_map, phext::to_coordinate(coord.as_str()), scroll.clone(), phext.clone());
    let phext_map = (*loaded_map).clone();
    let phext_buffer = phext::implode(phext_map);
    let _ = std::fs::write(phext, phext_buffer).unwrap();

    let length = output.len();
    let response =
        format!("{headers}\r\nContent-Length: {length}\r\n\r\n{output}");

    stream.write_all(response.as_bytes()).unwrap();
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
    println!("Serving {} bytes.", phext_buffer.len());

    loop {
        evt.wait(Timeout::Infinite)?;
        connection_id += 1;

        let parts = fetch_message(shmem.as_ptr(), length_offset);

        let command = phext::fetch(parts.as_str(), ps1);
        let argtemp = phext::fetch(parts.as_str(), ps2);
        let coordinate = phext::to_coordinate(argtemp.as_str());
        let update = phext::fetch(parts.as_str(), ps3);

        let mut scroll = String::new();
        let done = sq::process(connection_id, filename.clone(), &mut scroll, command, &mut phext_buffer, coordinate, update, argtemp.clone());
        let scroll_length = scroll.len();

        send_message(shmem.as_ptr(), length_offset, scroll);
        work.set(EventState::Signaled)?;
        let scroll_count = phext_buffer.len();
        println!("Sending {scroll_length} bytes to client #{connection_id} ({filename} contains {scroll_count} scrolls).");

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
            if std::fs::exists(SHARED_NAME).is_ok() {
                _ = std::fs::remove_file(SHARED_NAME);
            }
            if std::fs::exists(WORK_NAME).is_ok() {
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