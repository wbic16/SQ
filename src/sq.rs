use crate::phext;

pub fn args_required(command:&str) -> usize {
    if command == "shutdown" || command == "help" || command == "init" || command == "status" {
        return 2;
    }

    return 3;
}

pub fn process(connection_id: u64, source: String, scroll: &mut String, command: String, phext_buffer: &mut String, coordinate: phext::Coordinate, update: String, filename: String) -> bool {
    if command == "help" {
        *scroll = "
* help: display this online help screen
* status: display daemon statistics
* <file>: Hosts a new phext on startup if no daemon is running yet (creates a .sq directory)
* <port>: Starts sq in listening mode (bypassing daemon setup)
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
        *scroll = format!("Hosting: {}
Connection ID: {}
Phext Size: {}", source, connection_id, phext_buffer.len());
        return false;
    }

    if command == "checksum" {
        *scroll = phext::checksum(phext_buffer.as_str());
        return false;
    }

    if command == "select" || command == "pull" {
        *scroll = phext::fetch(phext_buffer.as_str(), coordinate);
        return false;
    }

    if command == "insert" {
        *scroll = format!("Inserted {} bytes", update.len());
        *phext_buffer = phext::insert(phext_buffer.clone(), coordinate, update.as_str());
        return false;
    }

    if command == "update" || command == "push" || command == "slurp" {
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
      *scroll = format!("Shutdown Initiated.");
      return true;
    }

    *scroll = format!("Unexpected command ignored.");
    return false;
}