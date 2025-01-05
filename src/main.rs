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
fn server(shmem: Shmem, _wkmem: Shmem) -> Result<(), Box<dyn std::error::Error>> {
    let mut connection_id: u64 = 0;
    let zeros = vec![0 as u8; SHARED_SEGMENT_SIZE];

    let (evt, _evt_used_bytes) = unsafe { Event::new(shmem.as_ptr(), true)? };
    //let (work, _used_work_bytes) = unsafe { Event::new(wkmem.as_ptr(), true) }.expect("wkmem error 1");

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
        println!("\tGot signal from Client #{connection_id}.");

        unsafe {
            let raw = std::slice::from_raw_parts(shmem.as_ptr().add(4), 20);
            let length_string = String::from_utf8_unchecked(raw.to_vec()).to_string();
            let length: usize = length_string.parse().unwrap_or(0);
            if length == 0 {
                println!("Ignoring invalid request.");
                continue;
            }
            let unparsed = std::slice::from_raw_parts(shmem.as_ptr().add(24), length);
            let parts = String::from_utf8_unchecked(unparsed.to_vec()).to_string();
            let command = phext::fetch(parts.as_str(), ps1);
            let argtemp = phext::fetch(parts.as_str(), ps2);
            let coordinate = phext::to_coordinate(argtemp.as_str());
            let update = phext::fetch(parts.as_str(), ps3);
            println!("Processing command='{}', coordinate='{}', update='{}'", command, coordinate, update);

            let mut scroll = String::new();
            let done = sq::process(&mut scroll, command, &mut phext_buffer, coordinate, update, argtemp.clone());

            std::ptr::copy_nonoverlapping(zeros.as_ptr(), shmem.as_ptr().add(4), SHARED_SEGMENT_SIZE-4);
            std::ptr::copy_nonoverlapping(scroll.as_ptr(), shmem.as_ptr().add(4), scroll.len());
            //work.set(EventState::Signaled)?;
            println!("Sending {}/{} bytes to client #{}.\n", scroll.len(), phext_buffer.len(), connection_id);

            if done {
                println!("Returning to the shell...");
                break;
            }
        }
    }

    println!("SQ Shutdown Complete.");
    Ok(())
}

// -----------------------------------------------------------------------------------------------------------
fn client(shmem: Shmem, wkmem: Shmem) -> Result<(), Box<dyn std::error::Error>> {
    let zeros = vec![0 as u8; SHARED_SEGMENT_SIZE];
    let (evt, evt_used_bytes) = unsafe { Event::from_existing(shmem.as_ptr())? };
    let (_work, work_used_bytes) = unsafe { Event::from_existing(wkmem.as_ptr())? };

    println!("Shared memory: {evt_used_bytes}, {work_used_bytes}.");

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
    let message = args.get(3).unwrap_or(&nothing);
    let mut encoded = String::new();
    encoded.push_str(command);
    encoded.push(phext::SCROLL_BREAK);
    encoded.push_str(coordinate);
    encoded.push(phext::SCROLL_BREAK);
    encoded.push_str(message);
    encoded.push(phext::SCROLL_BREAK);
    let prepared = format!("{:020}{}", encoded.len(), encoded);

    println!("Requesting {coordinate} from server (bytes={})...", encoded.len());
    unsafe {
        let zero_length = prepared.len() + 1;
        std::ptr::copy_nonoverlapping(zeros.as_ptr(), shmem.as_ptr().add(4), zero_length);
        std::ptr::copy_nonoverlapping(prepared.as_ptr(), shmem.as_ptr().add(4), prepared.len());
    }

    evt.set(EventState::Signaled)?;
    //work.wait(Timeout::Infinite)?;
    unsafe {
        let slice = std::slice::from_raw_parts(shmem.as_ptr().add(4), 1000);
        let response = String::from_utf8_lossy(slice).to_string();
        println!("{}: {}", coordinate, response);
    }

    println!("Client Done.");
    Ok(())
}
