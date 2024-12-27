
use crate::phext;

pub fn process(scroll: &mut String, command: String, phext_buffer: &mut String, coordinate: phext::Coordinate, update: String, filename: String) {
    if command == "select" {
        *scroll = phext::fetch(phext_buffer.as_str(), coordinate);
    } else if command == "insert" {
        *scroll = format!("Inserted {} bytes", update.len());
        *phext_buffer = phext::insert(phext_buffer.clone(), coordinate, update.as_str());
    } else if command == "update" {
        *scroll = format!("Updated {} bytes", update.len());
        *phext_buffer = phext::replace(phext_buffer.as_str(), coordinate, update.as_str());
    } else if command == "delete" {
        let old = phext::fetch(phext_buffer.as_str(), coordinate);
        *scroll = format!("Removed {} bytes", old.len());
        *phext_buffer = phext::replace(phext_buffer.as_str(), coordinate, "");
    } else if command == "save" {
        let _ = std::fs::write(filename.clone(), phext_buffer.as_str());
        *scroll = format!("Wrote {} bytes to {}", phext_buffer.len(), filename);
    }
}