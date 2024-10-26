use libphext::phext;
use raw_sync::{events::*, Timeout};
use shared_memory::*;
use std::env;

const MAX_MEM_SIZE: usize = 1024*1024; // 1 MiB

// -----------------------------------------------------------------------------------------------------------
fn fetch_source(filename: String) -> String {
    let message = format!("Unable to open {}", filename);
    let mut buffer:String = std::fs::read_to_string(filename).expect(&message);
    if buffer.len() > MAX_MEM_SIZE {
        buffer = buffer[0..MAX_MEM_SIZE].to_string();
    }
    return buffer;
}  

// -----------------------------------------------------------------------------------------------------------
fn main() -> Result<(), Box<dyn std::error::Error>> {    
        
    let shmem = match ShmemConf::new().size(MAX_MEM_SIZE).flink("phext_link").create() {
        Ok(m) => m,
        Err(ShmemError::LinkExists) => ShmemConf::new().flink("phext_link").open().expect("unable to locate phext_link"),
        Err(e) => return Err(Box::new(e)),
    };

    if shmem.is_owner() {
        let filename = env::args().nth(1).expect("Usage: sq.exe <phext>");
        println!("Loading {} into memory...", filename);
        
        let phext_buffer = fetch_source(filename);
        println!("Serving {} bytes.", phext_buffer.len());
        
        let (evt, _used_bytes) = unsafe {
            let shmem_ptr = shmem.as_ptr() as *mut u8;
            std::ptr::copy_nonoverlapping(phext_buffer.as_ptr(), shmem_ptr.add(4), phext_buffer.len());            
            Event::new(shmem.as_ptr(), true)
        }.expect("shmem error 1");

        loop {
            println!("Waiting for connection...");
            evt.wait(Timeout::Infinite)?;
            let shmem_ptr = shmem.as_ptr() as *mut u8;
            unsafe {
                let raw = std::slice::from_raw_parts(shmem_ptr.add(4), 100);
                let coord_string = String::from_utf8_lossy(raw).to_string();
                let coordinate = phext::to_coordinate(coord_string.as_str());
                println!("Fetching {}...", coord_string);
                let scroll = phext::fetch(phext_buffer.as_str(), coordinate) + "\0";
                std::ptr::copy_nonoverlapping(scroll.as_ptr(), shmem_ptr.add(4), scroll.len());
                println!("\nServiced client with {}/{} bytes.", scroll.len(), phext_buffer.len());
                evt.set(EventState::Signaled)?;
            }
        }

    } else {

        let coordinate = env::args().nth(1).expect("Usage: sq.exe <coordinate>") + "\0";

        println!("Opening SQ Connection...");
        let (evt, _used_bytes) = unsafe { Event::from_existing(shmem.as_ptr()) }.expect("shmem error 2");
        unsafe {
            let ptr = shmem.as_ptr();
            println!("Requesting {}...", coordinate.to_string());
            std::ptr::copy_nonoverlapping(coordinate.as_ptr(), ptr.add(4), coordinate.len());
            println!("Wrote {} bytes to shared memory.", coordinate.len());
            evt.set(EventState::Signaled)?;
            println!("Waiting for server response.");
            let _ = evt.wait(Timeout::Infinite);
            let slice = std::slice::from_raw_parts(ptr.add(4), 1000);
            let message = String::from_utf8_lossy(slice).to_string();
            println!("Server: {}", message);
        }
    }

    Ok(())
}
