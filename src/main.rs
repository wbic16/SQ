use libphext::phext;
use raw_sync::{events::*, Timeout};
use shared_memory::*;
use std::env;

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
fn server(shmem: Shmem, wkmem: Shmem) -> Result<(), Box<dyn std::error::Error>> {
    let mut connection_id: u64 = 0;

    let (evt, evt_used_bytes) = unsafe { Event::new(shmem.as_ptr(), true)? };
    let (work, _used_work_bytes) = unsafe { Event::new(wkmem.as_ptr(), true)? };

    let length_offset  = evt_used_bytes + 4;

    let ps1: phext::Coordinate = phext::to_coordinate("1.1.1/1.1.1/1.1.1");
    let ps2: phext::Coordinate = phext::to_coordinate("1.1.1/1.1.1/1.1.2");
    let ps3: phext::Coordinate = phext::to_coordinate("1.1.1/1.1.1/1.1.3");

    let mut filename = env::args().nth(1).expect("Usage: sq.exe <phext>");
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
        let done = sq::process(&mut scroll, command, &mut phext_buffer, coordinate, update, argtemp.clone());
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
    let usage = "Usage: sq.exe <command> <coordinate> <message>";
    if args.len() < 3 {
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
    println!("{}: {}", coordinate, response);

    Ok(())
}
