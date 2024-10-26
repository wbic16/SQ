use libphext::phext;
use raw_sync::{events::*, Timeout};
use shared_memory::*;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {    
    
    let mem_size: usize = 1024*1024; // 1 MiB
    let shmem = match ShmemConf::new().size(mem_size).flink("phext_link").create() {
        Ok(m) => m,
        Err(ShmemError::LinkExists) => ShmemConf::new().flink("phext_link").open().expect("unable to locate phext_link"),
        Err(e) => return Err(Box::new(e)),
    };

    if shmem.is_owner() {

        //let message = env::args().nth(1).expect("Usage: sq.exe <phext>");
        //println!("Loading phext document into memory...");
        // TODO: add a file parameter - load the phext doc on startup :)
        let message = "Hello World\x17Goodbye Plain Text.\x17Hello Multiverse";
        println!("Serving {} bytes.", message.len());
        
        let (evt, _used_bytes) = unsafe {
            let shmem_ptr = shmem.as_ptr() as *mut u8;
            std::ptr::copy_nonoverlapping(message.as_ptr(), shmem_ptr.add(4), message.len());
            Event::new(shmem.as_ptr(), true)
        }.expect("shmem error 1");

        loop {
            println!("Waiting for connection...");
            evt.wait(Timeout::Infinite)?;
            println!("\nServiced Client.");
        }

    } else {

        let coordinate = env::args().nth(1).expect("Usage: sq.exe <coordinate>");

        println!("Opening SQ Connection...");
        let (evt, _used_bytes) = unsafe { Event::from_existing(shmem.as_ptr()) }.expect("shmem error 2");
        unsafe {
            let ptr = shmem.as_ptr();
            let slice = std::slice::from_raw_parts(ptr.add(4), 1000);
            let test = String::from_utf8_lossy(slice).to_string();
            let doc = phext::fetch(test.as_str(), phext::to_coordinate(coordinate.as_str()));
            println!("Server: {}", doc);
        }
        evt.set(EventState::Signaled)?;
        println!("\tRequested.");

    }

    Ok(())
}
