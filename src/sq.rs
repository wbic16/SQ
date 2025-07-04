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

//------------------------------------------------------------------------------------------------------------
// process: performs the command line action for a given user request
//
// @param connection_id
// @param source
// @param scroll
// @param command
// @param phext_map
// @param coordinate
// @param update
// @param filename
//------------------------------------------------------------------------------------------------------------
pub fn process(connection_id: u64, source: String, scroll: &mut String, command: String, phext_map: &mut HashMap::<phext::Coordinate, String>, coordinate: phext::Coordinate, update: String, filename: String) -> bool {
    if command == "help" {
        *scroll = "
* help: display this online help screen
* status: display daemon statistics
* basic: launch a phext4d editor running on port 1337
* share <file>: Hosts a new phext on startup if no daemon is running yet (creates a .sq directory)
* host <port>: Starts sq in listening mode (bypassing daemon setup) - see the REST API reference
* toc: Dumps the current navigation table for the loaded phext
* get <file>: Returns the contents of the given phext in one response
* slurp <coord> <directory>: Creates a TOC for files in the given directory, and imports any plain-text files found
* diff <other>: Creates a phext-diff of the currently-loaded phext and other
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

    if command == "diff" {
        
        let compare = phext::implode(phext_map.clone());
        let diff = phext::subtract(update.as_str(), compare.as_str());
        *scroll = phext::textmap(diff.as_str());
        return false;
    }

    if command == "toc" {
        let buffer = phext::implode(phext_map.clone());
        *scroll = phext::textmap(buffer.as_str());
        return false;
    }

    if command == "get" {
        let message = "Unable to open requested phext ".to_string() + filename.as_str();
        let buffer:String = std::fs::read_to_string(filename).expect(&message);
        *scroll = buffer;
        return false;
    }

    if command == "checksum" {
        let serialized = phext::implode(phext_map.clone());
        *scroll = phext::checksum(serialized.as_str());
        return false;
    }

    if command == "delta" {
        let result = phext::manifest(phext::implode(phext_map.clone()).as_str());
        *scroll = result;
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

//------------------------------------------------------------------------------------------------------------
// csv_convert
//------------------------------------------------------------------------------------------------------------
pub fn csv_convert(csv: &str) -> HashMap::<phext::Coordinate, String>
{
    let parts = csv.split('\n');
    let mut coordinate = phext::to_coordinate("1.1.1/1.1.1/1.1.1");
    let mut result = HashMap::<phext::Coordinate, String>::new();
    for part in parts {
        let fields = part.split(',');
        for field in fields {
            result.insert(coordinate, field.to_string());
            coordinate.scroll_break();
        }
        coordinate.section_break();
    }
    return result;
}