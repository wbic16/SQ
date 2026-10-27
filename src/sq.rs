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
// coord_sort_key: extracts a 9-component tuple for deterministic ordering
//   hierarchy (highest to lowest): library, shelf, series, collection, volume, book, chapter, section, scroll
//------------------------------------------------------------------------------------------------------------
fn coord_sort_key(c: &phext::Coordinate) -> [usize; 9] {
    [c.z.library, c.z.shelf, c.z.series,
     c.y.collection, c.y.volume, c.y.book,
     c.x.chapter, c.x.section, c.x.scroll]
}

//------------------------------------------------------------------------------------------------------------
// delimiters_between: computes the minimal delimiter sequence to advance from `prev` to `curr`
//
// phext delimiter hierarchy (highest → lowest):
//   \x01  library break      resets shelf..scroll
//   \x1f  shelf break        resets series..scroll
//   \x1e  series break       resets collection..scroll
//   \x1d  collection break   resets volume..scroll
//   \x1c  volume break       resets book..scroll
//   \x1a  book break         resets chapter..scroll
//   \x19  chapter break      resets section..scroll
//   \x18  section break      resets scroll
//   \x17  scroll break       (lowest)
//
// When a higher-level break fires, all lower components reset to 1. We emit
// (curr[level] - prev[level]) copies of the break at the highest changed level,
// then (curr[lower] - 1) copies for each lower level.
//------------------------------------------------------------------------------------------------------------
fn delimiters_between(prev: &phext::Coordinate, curr: &phext::Coordinate) -> String {
    let p = coord_sort_key(prev);
    let c = coord_sort_key(curr);
    const DELIMS: [char; 9] = ['\x01', '\x1f', '\x1e', '\x1d', '\x1c', '\x1a', '\x19', '\x18', '\x17'];

    // Find the highest level that differs
    let mut level = 9usize; // sentinel: no difference
    for i in 0..9 {
        if p[i] != c[i] {
            level = i;
            break;
        }
    }

    if level >= 9 {
        return String::new(); // same coordinate
    }

    let mut result = String::new();

    // Emit delimiters at the changed level
    for _ in p[level]..c[level] {
        result.push(DELIMS[level]);
    }

    // Emit delimiters for all lower levels (reset from 1 → target)
    for i in (level + 1)..9 {
        for _ in 1..c[i] {
            result.push(DELIMS[i]);
        }
    }

    result
}

//------------------------------------------------------------------------------------------------------------
// implode_ref: borrow-only serialization of a phext map
//
// Produces the same byte sequence as phext::implode() but never clones the map.
// Only non-empty scrolls are emitted; empty scrolls are skipped (matching libphext behavior).
//------------------------------------------------------------------------------------------------------------
pub fn implode_ref(map: &HashMap<phext::Coordinate, String>) -> String {
    // Collect non-empty entries
    let mut entries: Vec<(&phext::Coordinate, &String)> = map.iter()
        .filter(|(_, v)| !v.is_empty())
        .collect();

    if entries.is_empty() {
        return String::new();
    }

    // Sort by coordinate hierarchy
    entries.sort_by(|a, b| coord_sort_key(a.0).cmp(&coord_sort_key(b.0)));

    // Pre-calculate total size to avoid reallocation
    let total_size: usize = entries.iter().map(|(_, v)| v.len()).sum::<usize>()
        + entries.len() * 9; // worst-case 9 delimiters per entry
    let mut result = String::with_capacity(total_size);

    let origin = phext::Coordinate::default();
    let mut prev = &origin;

    for (coord, content) in &entries {
        let delims = delimiters_between(prev, coord);
        result.push_str(&delims);
        result.push_str(content);
        prev = coord;
    }

    result
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
        // use implode_ref to avoid cloning the entire map just for .len()
        let buffer = implode_ref(phext_map);
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
        for ith in phext_map.iter() {
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
        // use implode_ref instead of cloning
        let compare = implode_ref(phext_map);
        let diff = phext::subtract(update.as_str(), compare.as_str());
        *scroll = phext::textmap(diff.as_str());
        return false;
    }

    if command == "toc" {
        // use implode_ref instead of cloning
        let buffer = implode_ref(phext_map);
        *scroll = phext::textmap(buffer.as_str());
        return false;
    }

    if command == "get" {
        let buffer: String = match std::fs::read_to_string(&filename) {
            Ok(b) => b,
            Err(e) => {
                *scroll = format!("Unable to open requested phext {}: {}", filename, e);
                return false;
            }
        };
        *scroll = buffer;
        return false;
    }

    if command == "checksum" {
        // use implode_ref instead of cloning
        let serialized = implode_ref(phext_map);
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
        // use implode_ref instead of cloning
        let output_buffer = implode_ref(phext_map);
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
// Coordinate Query API (v0.6.0)
//
//   GET  /api/v2/query?p=<phext>&from=<coord>&to=<coord>   inclusive coordinate range
//   GET  /api/v2/query?p=<phext>&set=<coord>,<coord>,...   explicit coordinate set
//   POST /api/v2/query?p=<phext>  (body = comma/newline-separated set, for large sets)
//
// Range and set may be combined; results are the union, deduplicated and sorted by
// coordinate (library → scroll). Range results include only non-empty scrolls; set
// results include every requested coordinate, with "" for empty ones. Optional
// `limit` caps the number of results.
//------------------------------------------------------------------------------------------------------------
pub struct QuerySpec {
    pub range: Option<(phext::Coordinate, phext::Coordinate)>,
    pub set: Vec<phext::Coordinate>,
    pub limit: Option<usize>,
}

/// Strict coordinate parser: exactly 9 positive integers as a.b.c/d.e.f/g.h.i
pub fn parse_coordinate_strict(text: &str) -> Option<phext::Coordinate> {
    let text = text.trim();
    let parts: Vec<&str> = text.split('/').collect();
    if parts.len() != 3 { return None; }
    for part in &parts {
        let nums: Vec<&str> = part.split('.').collect();
        if nums.len() != 3 { return None; }
        for n in nums {
            match n.parse::<usize>() {
                Ok(v) if v >= 1 => {}
                _ => return None,
            }
        }
    }
    Some(phext::to_coordinate(text))
}

pub fn parse_query(from: Option<&String>, to: Option<&String>, set: Option<&String>, limit: Option<&String>) -> Result<QuerySpec, String> {
    let from = from.map(|s| s.trim()).filter(|s| !s.is_empty());
    let to = to.map(|s| s.trim()).filter(|s| !s.is_empty());
    let range = match (from, to) {
        (Some(a), Some(b)) => {
            let start = parse_coordinate_strict(a).ok_or(format!("invalid 'from' coordinate: {}", a))?;
            let end = parse_coordinate_strict(b).ok_or(format!("invalid 'to' coordinate: {}", b))?;
            if coord_sort_key(&start) > coord_sort_key(&end) {
                return Err(format!("'from' ({}) is after 'to' ({})", a, b));
            }
            Some((start, end))
        }
        (None, None) => None,
        _ => return Err("a range needs both 'from' and 'to'".to_string()),
    };

    let mut coords = Vec::new();
    if let Some(raw) = set {
        for item in raw.split(|c| c == ',' || c == '\n' || c == ';') {
            let item = item.trim();
            if item.is_empty() { continue; }
            coords.push(parse_coordinate_strict(item).ok_or(format!("invalid coordinate in set: {}", item))?);
        }
    }

    if range.is_none() && coords.is_empty() {
        return Err("query needs a range (from + to) or a set of coordinates".to_string());
    }

    let limit = match limit.map(|s| s.trim()).filter(|s| !s.is_empty()) {
        None => None,
        Some(s) => Some(s.parse::<usize>().map_err(|_| format!("invalid limit: {}", s))?),
    };

    Ok(QuerySpec { range, set: coords, limit })
}

/// Runs a query against a loaded phext map and returns a JSON array of {coordinate, scroll}
pub fn run_query(phext_map: &HashMap<phext::Coordinate, String>, spec: &QuerySpec) -> String {
    let mut picked: std::collections::BTreeMap<[usize; 9], (phext::Coordinate, &str)> = std::collections::BTreeMap::new();

    if let Some((start, end)) = &spec.range {
        let lo = coord_sort_key(start);
        let hi = coord_sort_key(end);
        for (coord, scroll) in phext_map.iter() {
            if scroll.is_empty() { continue; }
            let key = coord_sort_key(coord);
            if key >= lo && key <= hi {
                picked.insert(key, (*coord, scroll.as_str()));
            }
        }
    }

    for coord in &spec.set {
        let scroll = phext_map.get(coord).map(|s| s.as_str()).unwrap_or("");
        picked.insert(coord_sort_key(coord), (*coord, scroll));
    }

    let take = spec.limit.unwrap_or(usize::MAX);
    let results: Vec<serde_json::Value> = picked.values().take(take)
        .map(|(coord, scroll)| serde_json::json!({
            "coordinate": coord.to_string(),
            "scroll": scroll,
        }))
        .collect();
    serde_json::Value::Array(results).to_string()
}
