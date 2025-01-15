use libphext::phext;
use raw_sync::{events::*, Timeout};
use shared_memory::*;
use std::env;
use std::net::TcpListener;
use std::io::BufRead;
use std::io::Write;
use std::collections::HashMap;

mod sq;
mod tests;

const SHARED_SEGMENT_SIZE: usize = 4*1024*1024; // 4 MB limit
const MAX_BUFFER_SIZE: usize = SHARED_SEGMENT_SIZE/2;
const WORK_SEGMENT_SIZE: usize = 1024;

const SHARED_NAME: &str = ".sq/link";
const WORK_NAME: &str = ".sq/work";

// -----------------------------------------------------------------------------------------------------------
fn fetch_source(filename: String) -> String {
    let message = format!("Unable to open {}", filename);
    let exists = std::fs::exists(filename.clone()).unwrap_or(false);
    if exists == false {
        let _ = std::fs::write(filename.clone(), "");
    }
    let mut buffer:String = std::fs::read_to_string(filename).expect(&message);

    if buffer.len() > MAX_BUFFER_SIZE {
        buffer = buffer[0..MAX_BUFFER_SIZE].to_string();
    }
    return buffer;
}

// -----------------------------------------------------------------------------------------------------------
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let exists = std::fs::exists(".sq").unwrap_or(false);
    if exists == false {
        let _ = std::fs::create_dir(".sq");
    }

    let phext_or_port = env::args().nth(1).unwrap_or("".to_string());
    let exists = std::fs::exists(phext_or_port.clone()).unwrap_or(false);
    let is_port_number = phext_or_port.parse::<u16>().is_ok();
    
    if exists == false && phext_or_port.len() > 0 && is_port_number {
        let port = phext_or_port;
        let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).unwrap();
        println!("Listening on port {port}...");

        let mut connection_id: u64 = 0;
        for stream in listener.incoming() {
            let stream = stream.unwrap();
            connection_id += 1;
            handle_tcp_connection(connection_id, stream);
        }
        return Ok(());
    }

    let error_message_link = format!("unable to link {}", SHARED_NAME);
    let error_message_work = format!("unable to work {}", WORK_NAME);

    let shmem = match ShmemConf::new().size(SHARED_SEGMENT_SIZE).flink(SHARED_NAME).create() {
        Ok(m) => m,
        Err(ShmemError::LinkExists) => ShmemConf::new().flink(SHARED_NAME).open().expect(error_message_link.as_str()),
        Err(e) => return Err(Box::new(e)),
    };
    let wkmem = match ShmemConf::new().size(WORK_SEGMENT_SIZE).flink(WORK_NAME).create() {
        Ok(m) => m,
        Err(ShmemError::LinkExists) => ShmemConf::new().flink(WORK_NAME).open().expect(error_message_work.as_str()),
        Err(e) => return Err(Box::new(e)),
    };

    if shmem.is_owner() { return server(shmem, wkmem); }
    else                { return client(shmem, wkmem); }
}

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
fn url_decode(encoded: &str) -> String {
    let stage1 = encoded.to_string().replace("+", " ");
    let stage2 = percent_encoding::percent_decode(stage1.as_bytes())
        .decode_utf8_lossy()
        .to_string();
    
    return stage2;
}

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
fn request_parse(request: &str) -> HashMap<String, String> {
    let mut result = HashMap::new();
    let mut parts = request.splitn(2, '?');
    if let (Some(_key), Some(value)) = (parts.next(), parts.next()) {
        result = parse_query_string(value.strip_suffix(" HTTP/1.1").unwrap_or(value));
    }

    return result;
}

// -----------------------------------------------------------------------------------------------------------
fn handle_tcp_connection(connection_id: u64, mut stream: std::net::TcpStream) {
    let buf_reader = std::io::BufReader::new(&stream);
    let http_request: Vec<_> = buf_reader
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();
    let request = &http_request[0];
    if request.starts_with("GET ") == false {
        stream.write_all("HTTP/1.1 400 Bad Request\r\n".as_bytes()).unwrap();
        println!("Ignoring {}", request);
        return;
    }
    println!("Request: {}", request);

    let headers = "HTTP/1.1 200 OK";
    let parsed = request_parse(request);
    let nothing = String::new();
    let scroll = parsed.get("s").unwrap_or(&nothing);
    let coord  = parsed.get("c").unwrap_or(&nothing);
    let phext  = parsed.get("p").unwrap_or(&nothing).to_owned() + ".phext";
    let mut phext_buffer = fetch_source(phext.clone());
    //let mut title = "UNKNOWN";
    let mut output = String::new();
    let mut command = String::new();
    //let mut action = String::new();
    if request.starts_with("GET /api/v2/select") {
        command = "select".to_string();
        //title = "SELECT";
        //action = format!("Selected...{coord} from {phext}.");
    } else if request.starts_with("GET /api/v2/insert") {
        command = "insert".to_string();
        //title = "INSERT";
        //action = format!("Inserted {scroll} at {coord} into {phext}.");
    } else if request.starts_with("GET /api/v2/update") {
        command = "update".to_string();
        //title = "UPDATE";
        //action = format!("Updated {phext}::{coord} = {scroll}.");
    } else if request.starts_with("GET /api/v2/delete") {
        command = "delete".to_string();
        //title = "DELETE";
        //action = format!("Removed scroll content at {coord} from {phext}.");
    }
    
    let _ = sq::process(connection_id, phext.clone(), &mut output, command, &mut phext_buffer, phext::to_coordinate(coord.as_str()), scroll.clone(), nothing);
    let _ = std::fs::write(phext, phext_buffer).unwrap();

    let length = output.len();
    let response =
        format!("{headers}\r\nContent-Length: {length}\r\n\r\n{output}");

    stream.write_all(response.as_bytes()).unwrap();
}

// -----------------------------------------------------------------------------------------------------------
fn server(shmem: Shmem, wkmem: Shmem) -> Result<(), Box<dyn std::error::Error>> {
    let mut connection_id: u64 = 0;

    let (evt, evt_used_bytes) = unsafe { Event::new(shmem.as_ptr(), true)? };
    let (work, _used_work_bytes) = unsafe { Event::new(wkmem.as_ptr(), true)? };

    let length_offset  = evt_used_bytes + 4;

    let ps1: phext::Coordinate = phext::to_coordinate("1.1.1/1.1.1/1.1.1");
    let ps2: phext::Coordinate = phext::to_coordinate("1.1.1/1.1.1/1.1.2");
    let ps3: phext::Coordinate = phext::to_coordinate("1.1.1/1.1.1/1.1.3");

    let mut filename = env::args().nth(1).expect("Usage: sq.exe <phext>|<port>");    

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
        println!("Sending {}/{} bytes to client #{}.", scroll_length, phext_buffer.len(), connection_id);

        if done {
            println!("Returning to the shell...");
            break;
        }
    }

    println!("SQ Shutdown Complete.");
    Ok(())
}

// -----------------------------------------------------------------------------------------------------------
fn client(shmem: Shmem, wkmem: Shmem) -> Result<(), Box<dyn std::error::Error>> {
    let (evt, evt_used_bytes) = unsafe { Event::from_existing(shmem.as_ptr())? };
    let (work, _work_used_bytes) = unsafe { Event::from_existing(wkmem.as_ptr())? };
    let length_offset  = evt_used_bytes + 4;

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

    let coordinate = args.get(2).unwrap_or(&nothing);
    let mut message: String = args.get(3).unwrap_or(&nothing).to_string();
    if command == "push" {
        message = fetch_source(message);
    }
    let mut encoded = String::new();
    encoded.push_str(command);
    encoded.push(phext::SCROLL_BREAK);
    encoded.push_str(coordinate);
    encoded.push(phext::SCROLL_BREAK);
    encoded.push_str(message.as_str());
    encoded.push(phext::SCROLL_BREAK);

    send_message(shmem.as_ptr(), length_offset, encoded);

    evt.set(EventState::Signaled)?;
    work.wait(Timeout::Infinite)?;
    let mut response = fetch_message(shmem.as_ptr(), length_offset);
    if command == "pull" {
        let filename = message;
        let _ = std::fs::write(filename.clone(), response.clone());
        response = format!("Exported scroll at {coordinate} to {filename}.").to_string();
    }
    if coordinate.len() > 0 {
        println!("{coordinate}: {response}");
    } else {
        println!("{response}");
    }

    Ok(())
}
