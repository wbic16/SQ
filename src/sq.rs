
use crate::phext;

pub fn process(scroll: &mut String, command: String, phext_buffer: &mut String, coordinate: phext::Coordinate, update: String, filename: String) -> bool {
    if command == "help" {
        *scroll = "* help: display this online help screen
* select: fetch a scroll of text from the loaded phext
* insert: append text to the specified scroll
* update: overwrite text at the specified scroll
* delete: truncates the specified scroll
* save: dumps the contents of the loaded phext to disk
* shutdown: terminate the phext server".to_string();
        return false;
    }
    
    if command == "select" {
        *scroll = phext::fetch(phext_buffer.as_str(), coordinate);
        return false;
    }
    
    if command == "insert" {
        *scroll = format!("Inserted {} bytes", update.len());
        *phext_buffer = phext::insert(phext_buffer.clone(), coordinate, update.as_str());
        return false;
    }
    
    if command == "update" {
        *scroll = format!("Updated {} bytes", update.len());
        *phext_buffer = phext::replace(phext_buffer.as_str(), coordinate, update.as_str());
        return false;
    }
    
    if command == "delete" {
        let old = phext::fetch(phext_buffer.as_str(), coordinate);
        *scroll = format!("Removed {} bytes", old.len());
        *phext_buffer = phext::replace(phext_buffer.as_str(), coordinate, "");
        return false;
    }
    
    if command == "save" {
        let _ = std::fs::write(filename.clone(), phext_buffer.as_str());
        *scroll = format!("Wrote {} bytes to {}", phext_buffer.len(), filename);
        return false;
    }

    if command == "shutdown" {
      *scroll = format!("Instructed daemon to terminate.");
      return true;      
    }

    *scroll = format!("Unexpected command ignored.");
    return false;
}