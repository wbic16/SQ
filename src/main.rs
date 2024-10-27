use libphext::phext;
use raw_sync::{events::*, Timeout};
use shared_memory::*;
use std::env;

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

    let error_message = format!("unable to locate {}", shared_name);
    let error_message_work = format!("unable to locate {}", work_name);

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
        let filename = env::args().nth(1).expect("Usage: sq.exe <phext>");
        println!("Loading {} into memory...", filename);
        
        let phext_buffer = fetch_source(filename.clone());
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
            println!("Client #{}", connection_id);
            unsafe {
                let raw = std::slice::from_raw_parts(shmem.as_ptr().add(4), 100);
                let coord_string = String::from_utf8_lossy(raw).to_string();
                let coordinate = phext::to_coordinate(coord_string.as_str());
                println!("Fetching {} from {}...", coord_string, filename);

                let scroll = phext::fetch(phext_buffer.as_str(), coordinate);
                std::ptr::copy_nonoverlapping(zeros.as_ptr(), shmem.as_ptr().add(4), SHARED_SEGMENT_SIZE-4);
                std::ptr::copy_nonoverlapping(scroll.as_ptr(), shmem.as_ptr().add(4), scroll.len());
                work.set(EventState::Signaled)?;
                println!("\nServiced client with {}/{} bytes.", scroll.len(), phext_buffer.len());
            }
        }
    } else {
        let coordinate = env::args().nth(1).expect("Usage: sq.exe <coordinate>");
        println!("Contacting SQ...");

        let (evt, _used_bytes) = unsafe { Event::from_existing(shmem.as_ptr()) }.expect("failed to open SQ connection (1)");
        let (work, _used_work_bytes) = unsafe { Event::from_existing(wkmem.as_ptr()) }.expect("failed to open SQ connection (2)");

        unsafe {
            std::ptr::copy_nonoverlapping(zeros.as_ptr(), shmem.as_ptr().add(4), SHARED_SEGMENT_SIZE-4);
            std::ptr::copy_nonoverlapping(coordinate.as_ptr(), shmem.as_ptr().add(4), coordinate.len());
            evt.set(EventState::Signaled)?;
            println!("Requested {}", coordinate);
            work.wait(Timeout::Infinite)?;
            let slice = std::slice::from_raw_parts(shmem.as_ptr().add(4), 1000);
            let message = String::from_utf8_lossy(slice).to_string();
            println!("Scroll {}: {}", coordinate, message);
        }
    }

    println!("Done !");
    Ok(())
}