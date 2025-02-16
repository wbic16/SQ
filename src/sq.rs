//------------------------------------------------------------------------------------------------------------
// file: sq.rs
// purpose: defines the high-level commands available in daemon mode
//
// SQ leverages libphext-rs to provide a minimal hierarchical database.
//------------------------------------------------------------------------------------------------------------
use crate::phext;
use std::collections::HashMap;

pub fn args_required(command:&str) -> usize {
    if command == "shutdown" ||
       command == "help" ||
       command == "init" ||
       command == "status" ||
       command == "toc" {
        return 2;
    }

    return 3;
}

pub fn process(connection_id: u64, source: String, scroll: &mut String, command: String, phext_map: &mut HashMap::<phext::Coordinate, String>, coordinate: phext::Coordinate, update: String, filename: String) -> bool {
    if command == "help" {
        *scroll = "
* help: display this online help screen
* status: display daemon statistics
* <file>: Hosts a new phext on startup if no daemon is running yet (creates a .sq directory)
* <port>: Starts sq in listening mode (bypassing daemon setup) - see the REST API reference
* toc: Dumps the current navigation table for the loaded phext
* slurp <coord> <directory>: Creates a TOC for files in the given directory, and imports any plain-text files found
* push <coord> <file>: Imports a file into your phext at the given coordinate
* pull <coord> <file>: Exports a scroll to a file of your choice
* select <coord>: fetch a scroll of text from the loaded phext
* insert <coord> \"text\": append text to the specified scroll
* update <coord> \"text\": overwrite text at the specified scroll
* delete <coord>: truncates the specified scroll
* save <file>: dumps the contents of the loaded phext to disk
* shutdown: terminate the phext server".to_string();
        return false;
    }

    if command == "status" {
        let buffer = phext::implode(phext_map.clone());
        *scroll = format!("Hosting: {}
Connection ID: {}
Phext Size: {}
Scrolls: {}", source, connection_id, buffer.len(), phext_map.iter().size_hint().0);
        return false;
    }

    if command == "toc" {
        let buffer = phext::implode(phext_map.clone());
        *scroll = phext::textmap(buffer.as_str());
        return false;
    }

    if command == "checksum" {
        let serialized = phext::implode(phext_map.clone());
        *scroll = phext::checksum(serialized.as_str());
        return false;
    }

    if command == "select" || command == "pull" {
        if phext_map.contains_key(&coordinate) {
            let nothing = String::new();
            *scroll = phext_map.get(&coordinate).unwrap_or(&nothing).clone();
        } else {
            *scroll = String::new();
        }
        return false;
    }

    if command == "insert" {
        *scroll = format!("Inserted {} bytes", update.len());
        let mut concatenated = String::new();
        if phext_map.contains_key(&coordinate) {
            let nothing = String::new();
            concatenated = phext_map.get(&coordinate).unwrap_or(&nothing).clone()
        }
        concatenated.push_str(update.as_str());
        (*phext_map).insert(coordinate, concatenated);
        return false;
    }

    if command == "update" || command == "push" || command == "slurp" {
        *scroll = format!("Updated {} bytes", update.len());
        phext_map.insert(coordinate, update);
        return false;
    }

    if command == "delete" {
        let mut old = String::new();
        if phext_map.contains_key(&coordinate) {
            let nothing = String::new();
            old = phext_map.get(&coordinate).unwrap_or(&nothing).clone();
            phext_map.remove(&coordinate);
        }
        *scroll = format!("Removed {} bytes", old.len());
        return false;
    }

    if command == "save" {
        let output_buffer = phext::implode(phext_map.clone());
        let _ = std::fs::write(filename.clone(), output_buffer.as_str());
        *scroll = format!("Wrote {} bytes to {}", output_buffer.len(), filename);
        return false;
    }

    if command == "load" {
        *scroll = format!("Loaded {filename}");
        return false;
    }

    if command == "shutdown" {
      *scroll = format!("Shutdown Initiated.");
      return true;
    }

    *scroll = format!("Unexpected command ignored.");
    return false;
}