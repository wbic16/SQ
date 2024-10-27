use libphext::phext;
use raw_sync::{events::*, Timeout};
use shared_memory::*;
use std::env;

const EVENT_SEGMENT_SIZE: usize = 1024;     // evt
const CONNECT_SEGMENT_SIZE: usize = 1024;   // pdm
const WORK_SEGMENT_SIZE: usize = 1024*1024; // work

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
        
    let shmem = match ShmemConf::new().size(EVENT_SEGMENT_SIZE).flink("phext_link").create() {
        Ok(m) => m,
        Err(ShmemError::LinkExists) => ShmemConf::new().flink("phext_link").open().expect("unable to locate phext_link"),
        Err(e) => return Err(Box::new(e)),
    };
    let mut connection_id: u64 = 0;

    if shmem.is_owner() {
        let filename = env::args().nth(1).expect("Usage: sq.exe <phext>");
        println!("Loading {} into memory...", filename);
        
        let phext_buffer = fetch_source(filename);
        println!("Serving {} bytes.", phext_buffer.len());

        let pdmem = match ShmemConf::new().size(CONNECT_SEGMENT_SIZE).flink("phext_daemon").create() {
            Ok(m) => m,
            Err(ShmemError::LinkExists) => ShmemConf::new().flink("phext_daemon").open().expect("unable to locate phext_daemon"),
            Err(e) => return Err(Box::new(e)),
        };
        let workmem = match ShmemConf::new().size(WORK_SEGMENT_SIZE).flink("phext_work").create() {
            Ok(m) => m,
            Err(ShmemError::LinkExists) => ShmemConf::new().flink("phext_work").open().expect("unable to locate phext_work"),
            Err(e) => return Err(Box::new(e)),
        };
        
        let (evt, _used_evt_bytes) = unsafe {
            let shmem_ptr = shmem.as_ptr() as *mut u8;
            std::ptr::copy_nonoverlapping(phext_buffer.as_ptr(), shmem_ptr.add(4), phext_buffer.len());            
            Event::new(shmem.as_ptr(), true)
        }.expect("shmem error 1");

        let (pdm, _used_pdm_bytes) = unsafe {
            let pdmem_ptr = pdmem.as_ptr() as *mut u8;
            let cid = connection_id.to_string();
            std::ptr::copy_nonoverlapping(cid.as_ptr(), pdmem_ptr, cid.len());
            Event::new(pdmem.as_ptr(), true)
        }.expect("pdmem error 1");

        let (work, _used_work_bytes) = unsafe {
            let workmem_ptr = workmem.as_ptr() as *mut u8;
            let initial = "\0\0\0\0";
            std::ptr::copy_nonoverlapping(initial.as_ptr(), workmem_ptr, initial.len());
            Event::new(workmem.as_ptr(), true)
        }.expect("workmem error 1");

        loop {
            println!("Waiting for connection...");
            let cid = connection_id.to_string();
            unsafe {
                std::ptr::copy_nonoverlapping(cid.as_ptr(), pdmem.as_ptr(), cid.len());
            }
            pdm.wait(Timeout::Infinite)?;
            println!("New Client: {}", connection_id);
            connection_id += 1;
            evt.wait(Timeout::Infinite)?;
            unsafe {
                let raw = std::slice::from_raw_parts(shmem.as_ptr().add(4), 100);
                let coord_string = String::from_utf8_lossy(raw).to_string();
                let coordinate = phext::to_coordinate(coord_string.as_str());
                println!("Fetching {}...", coord_string);
                let scroll = phext::fetch(phext_buffer.as_str(), coordinate) + "\0";
                std::ptr::copy_nonoverlapping(scroll.as_ptr(), workmem.as_ptr().add(4), scroll.len());
                println!("\nServiced client with {}/{} bytes.", scroll.len(), phext_buffer.len());
                work.set(EventState::Signaled)?;
            }
        }

    } else {

        let coordinate = env::args().nth(1).expect("Usage: sq.exe <coordinate>") + "\0";
        let pdmem = match ShmemConf::new().size(CONNECT_SEGMENT_SIZE).flink("phext_daemon").create() {
            Ok(m) => m,
            Err(ShmemError::LinkExists) => ShmemConf::new().flink("phext_daemon").open().expect("unable to locate phext_daemon"),
            Err(e) => return Err(Box::new(e)),
        };
        let workmem = match ShmemConf::new().size(WORK_SEGMENT_SIZE).flink("phext_work").create() {
            Ok(m) => m,
            Err(ShmemError::LinkExists) => ShmemConf::new().flink("phext_work").open().expect("unable to locate phext_work"),
            Err(e) => return Err(Box::new(e)),
        };

        let (evt, _used_bytes) = unsafe { Event::from_existing(shmem.as_ptr()) }.expect("shmem error 2");
        let (pdm, _used_pdm_bytes) = unsafe { Event::from_existing(pdmem.as_ptr()) }.expect("pdmem error 2");
        let (work, _used_work_bytes) = unsafe { Event::from_existing(workmem.as_ptr()) }.expect("workmem error 2");
        unsafe {
            println!("Opening SQ Connection...");
            std::ptr::copy_nonoverlapping(coordinate.as_ptr(), pdmem.as_ptr().add(4), coordinate.len());
            pdm.set(EventState::Signaled)?;            
            evt.set(EventState::Signaled);
            println!("Requested {}", coordinate.to_string());
            work.wait(Timeout::Infinite)?;
            let _ = evt.wait(Timeout::Infinite);
            let slice = std::slice::from_raw_parts(workmem.as_ptr().add(4), 1000);
            let message = String::from_utf8_lossy(slice).to_string();
            println!("Server: {}", message);
        }
    }

    Ok(())
}
