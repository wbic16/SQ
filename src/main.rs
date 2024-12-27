use libphext::phext;
use raw_sync::{events::*, Timeout};
use shared_memory::*;
use std::env;

mod sq;
mod tests;

const SHARED_SEGMENT_SIZE: usize = 2*1024*1024; // work
const MAX_BUFFER_SIZE: usize = SHARED_SEGMENT_SIZE/2;
const WORK_SEGMENT_SIZE: usize = 1024;

// -----------------------------------------------------------------------------------------------------------
fn fetch_source(filename: String) -> String {
    let message = format!("Unable to open {}", filename);
    let mut buffer:String = std::fs::read_to_string(filename).expect(&message);

    if buffer.len() > MAX_BUFFER_SIZE {
        buffer = buffer[0..MAX_BUFFER_SIZE].to_string();
    }
    return buffer;
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let zeros = vec![0 as u8; SHARED_SEGMENT_SIZE];
    let shared_name = "phext_link";
    let work_name = "phext_work";

    let ps1 = phext::to_coordinate("1.1.1/1.1.1/1.1.1");
    let ps2 = phext::to_coordinate("1.1.1/1.1.1/1.1.2");
    let ps3 = phext::to_coordinate("1.1.1/1.1.1/1.1.3");

    let error_message = format!("unable to link {}", shared_name);
    let error_message_work = format!("unable to work {}", work_name);

    let shmem = match ShmemConf::new().size(SHARED_SEGMENT_SIZE).flink(shared_name).create() {
        Ok(m) => m,
        Err(ShmemError::LinkExists) => ShmemConf::new().flink(shared_name).open().expect(error_message.as_str()),
        Err(e) => return Err(Box::new(e)),
    };
    let wkmem = match ShmemConf::new().size(WORK_SEGMENT_SIZE).flink(work_name).create() {
        Ok(m) => m,
        Err(ShmemError::LinkExists) => ShmemConf::new().flink(work_name).open().expect(error_message_work.as_str()),
        Err(e) => return Err(Box::new(e)),
    };
    let mut connection_id: u64 = 0;

    if shmem.is_owner() {
        // server
        let filename = env::args().nth(1).expect("Usage: sq.exe <phext>");
        println!("Loading {} into memory...", filename);

        let mut phext_buffer = fetch_source(filename.clone());
        println!("Serving {} bytes.", phext_buffer.len());

        let (evt, _used_evt_bytes) = unsafe {
            Event::new(shmem.as_ptr(), true)
        }.expect("shmem error 1");
        let (work, _used_work_bytes) = unsafe {
            Event::new(wkmem.as_ptr(), true)
        }.expect("shmem error 1");

        loop {
            println!("Waiting for connection...");
            evt.wait(Timeout::Infinite)?;
            connection_id += 1;
            unsafe {
                let raw = std::slice::from_raw_parts(shmem.as_ptr().add(4), 20);
                let length_string = String::from_utf8_unchecked(raw.to_vec()).to_string();
                let length: usize = length_string.parse().unwrap();
                let unparsed = std::slice::from_raw_parts(shmem.as_ptr().add(24), length);                
                let parts = String::from_utf8_unchecked(unparsed.to_vec()).to_string();
                let command = phext::fetch(parts.as_str(), ps1);
                let argtemp = phext::fetch(parts.as_str(), ps2);
                let coordinate = phext::to_coordinate(argtemp.as_str());
                let update = phext::fetch(parts.as_str(), ps3);
                println!("Processing command='{}', coordinate='{}', update='{}'", command, coordinate, update);

                let mut scroll = String::new();
                sq::process(&mut scroll, command, &mut phext_buffer, coordinate, update, argtemp.clone());

                std::ptr::copy_nonoverlapping(zeros.as_ptr(), shmem.as_ptr().add(4), SHARED_SEGMENT_SIZE-4);
                std::ptr::copy_nonoverlapping(scroll.as_ptr(), shmem.as_ptr().add(4), scroll.len());
                work.set(EventState::Signaled)?;
                println!("Sending {}/{} bytes to client #{}.\n", scroll.len(), phext_buffer.len(), connection_id);
            }
        }
    } else {
        // client
        let args: Vec<String> = env::args().collect();
        let usage = "Usage: sq.exe <command> <coordinate> <message>";
        if args.len() < 3 {
            println!("{}", usage);
            return Ok(());
        }
        let nothing: String = String::new();
        let command = args.get(1).unwrap_or(&nothing);
        let coordinate = args.get(2).unwrap_or(&nothing);
        let message = args.get(3).unwrap_or(&nothing);
        let mut encoded = String::new();
        encoded.push_str(command);
        encoded.push('\x17');
        encoded.push_str(coordinate);
        encoded.push('\x17');
        encoded.push_str(message);
        encoded.push('\x17');
        let prepared = format!("{:020}{}", encoded.len(), encoded);

        let (evt, _used_bytes) = unsafe { Event::from_existing(shmem.as_ptr()) }.expect("failed to open SQ connection (1)");
        let (work, _used_work_bytes) = unsafe { Event::from_existing(wkmem.as_ptr()) }.expect("failed to open SQ connection (2)");

        unsafe {
            std::ptr::copy_nonoverlapping(zeros.as_ptr(), shmem.as_ptr().add(4), SHARED_SEGMENT_SIZE-4);
            std::ptr::copy_nonoverlapping(prepared.as_ptr(), shmem.as_ptr().add(4), prepared.len());
            evt.set(EventState::Signaled)?;
            work.wait(Timeout::Infinite)?;
            let slice = std::slice::from_raw_parts(shmem.as_ptr().add(4), 1000);
            let response = String::from_utf8_lossy(slice).to_string();
            println!("{}: {}", coordinate, response);
        }
    }

    Ok(())
}