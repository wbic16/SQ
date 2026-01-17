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
// json_escape: simple wrapper to avoid breaking json-export
//------------------------------------------------------------------------------------------------------------
fn json_escape(input: String) -> String {
    let mut result = input;
    result = result.replace('"', "\\\"");
    result = result.replace('\n', "\\n");
    return result;
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
// @param algorithm - hash algorithm to use for coordinate inference
// @param limit - minimum scroll length for XOR hashing
//------------------------------------------------------------------------------------------------------------
pub fn process(connection_id: u64, source: String, scroll: &mut String, command: String, phext_map: &mut HashMap::<phext::Coordinate, String>, coordinate: phext::Coordinate, update: String, filename: String, algorithm: crate::HashAlgorithm, limit: usize) -> bool {
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

    if command == "version" {
        *scroll = format!("{}", env!("CARGO_PKG_VERSION"));
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

    if command == "json-export" {
        let mut result = String::new();
        result += "[\n";
        let mut started = false;
        for ith in phext_map {
            if !started {
                started = true;
            } else { result += ","; }
            result += &format!("   {{ \"coord\": \"{}\", \"scroll\": \"{}\" }}\n",
                json_escape(ith.0.to_string()), 
                json_escape(ith.1.to_string())).to_string();
        }
        result += "]\n";
        *scroll = result.clone();
        let json_filename = format!("{}.json", filename);
        let _ = std::fs::write(json_filename, result);
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
        let mut diff_map: HashMap<phext::Coordinate, String> = Default::default();
        let mut output:HashMap<phext::Coordinate, String> = Default::default();
        for line in update.lines() {
            let parsed:Vec<&str> = line.split(": ").collect();
            if parsed.len() == 0 { continue; }
            let parsed_coordinate = phext::to_coordinate(parsed[0]);
            if parsed_coordinate.validate_coordinate() && parsed.len() > 1 {
                let parsed_hash = parsed[1];
                diff_map.insert(parsed_coordinate, parsed_hash.to_string());
            }
        }
        for key in phext_map.keys() {
            let checksum = phext::checksum(phext_map[key].as_str());
            if diff_map.contains_key(key) == false || checksum != diff_map[key] {
                output.insert(key.clone(), phext_map[key].clone());
            }
        }
        for key in diff_map.keys() {
            if phext_map.contains_key(key) == false {
                output.insert(key.clone(), "---sq:Scroll-Missing---".to_string());
            }
        }
        *scroll = phext::implode(output);
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

    if command == "where" {
        println!("Processing where");
        let algo_name = match algorithm {
            crate::HashAlgorithm::Xor => "xor",
            crate::HashAlgorithm::Checksum => "checksum",
        };
        let computed = crate::infer_coordinate(update.as_str(), limit, algorithm);
        *scroll = format!("Calculated coordinate {} for input (algo={}, limit={}).", computed, algo_name, limit);
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