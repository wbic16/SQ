//------------------------------------------------------------------------------------------------------------
// file: seed.rs
// purpose: Seed cache + cache-aware fetch for phext hosting
//
// A seed cache is the hash envelope of a phext: for every populated coordinate,
// the hash of its scroll — and nothing else. Its size is a fixed constant times
// the number of scrolls, independent of how large any scroll (or the whole
// document) is, and the envelope compresses well (coordinates are sorted and
// clustered). A client downloads the seed on load, builds its navigation
// locally, and then streams scrolls in as they are viewed, verifying each one
// against the seed before caching it.
//
// Hash function (Will, 2026-11-10): xxh3_128 — libphext's phext::checksum,
// the same per-scroll hash the delta command already diffs with. Encoded as
// 32 lowercase hex characters (the 16-byte digest, zero-padded), exactly as
// phext::checksum formats it.
//
// Cache-aware fetch protocol: the client presents three things per scroll —
//   (1) the coordinate, (2) its current hash for that scroll, and
//   (3) a flag saying whether it holds the content locally.
// The server answers with a small "unchanged" confirmation (coordinate + hash,
// no content) ONLY when the client holds the content AND its hash matches the
// server's current hash. In every other case — no local content, or a stale
// hash — the fetch runs as normal: full content plus the current hash.
//
// Routes (same auth treatment as the neighboring read routes):
//   GET  /api/v2/seed?p=<phext>
//   GET  /api/v2/fetch?p=<phext>&c=<coord>&h=<hash>&has=<0|1>
//   POST /api/v2/fetch?p=<phext>   body: JSON array of
//        {"coordinate": "...", "hash": "...", "has_content": true|false}
//
// The client half of this file (ScrollCache + fetch_cached + the sq seed /
// sq fetch CLI helpers) is the reference consumer of the protocol.
//------------------------------------------------------------------------------------------------------------

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use crate::phext;

//------------------------------------------------------------------------------------------------------------
// coordinate ordering key (mirrors sq.rs coord_sort_key):
// library, shelf, series, collection, volume, book, chapter, section, scroll
//------------------------------------------------------------------------------------------------------------
fn sort_key(c: &phext::Coordinate) -> [usize; 9] {
    [c.z.library, c.z.shelf, c.z.series,
     c.y.collection, c.y.volume, c.y.book,
     c.x.chapter, c.x.section, c.x.scroll]
}

//------------------------------------------------------------------------------------------------------------
// the hash of one scroll: xxh3_128 via libphext, 32 lowercase hex chars
//------------------------------------------------------------------------------------------------------------
pub fn scroll_hash(content: &str) -> String {
    phext::checksum(content)
}

//------------------------------------------------------------------------------------------------------------
// build_seed: the seed envelope for a loaded phext map, as a JSON string
//
//   {"algorithm":"xxh3_128","scrolls":N,
//    "hashes":[{"coordinate":"1.1.1/1.1.1/1.1.1","hash":"<32 hex>"}, ...]}
//
// Only populated (non-empty) scrolls appear — the same set implode_ref emits.
// Entries are sorted by coordinate, library down to scroll (query's ordering).
//------------------------------------------------------------------------------------------------------------
pub fn build_seed(phext_map: &HashMap<phext::Coordinate, String>) -> String {
    let mut entries: Vec<(&phext::Coordinate, &String)> = phext_map.iter()
        .filter(|(_, v)| !v.is_empty())
        .collect();
    entries.sort_by(|a, b| sort_key(a.0).cmp(&sort_key(b.0)));

    let hashes: Vec<serde_json::Value> = entries.iter()
        .map(|(coord, content)| serde_json::json!({
            "coordinate": coord.to_string(),
            "hash": scroll_hash(content),
        }))
        .collect();

    serde_json::json!({
        "algorithm": "xxh3_128",
        "scrolls": entries.len(),
        "hashes": hashes,
    }).to_string()
}

//------------------------------------------------------------------------------------------------------------
// parse_seed: a seed envelope back into coordinate -> hash
//------------------------------------------------------------------------------------------------------------
pub fn parse_seed(seed_json: &str) -> Result<HashMap<String, String>, String> {
    let value: serde_json::Value = serde_json::from_str(seed_json)
        .map_err(|e| format!("invalid seed envelope: {}", e))?;
    let list = value.get("hashes").and_then(|h| h.as_array())
        .ok_or("seed envelope is missing its 'hashes' array")?;
    let mut result = HashMap::new();
    for entry in list {
        let coord = entry.get("coordinate").and_then(|c| c.as_str());
        let hash = entry.get("hash").and_then(|h| h.as_str());
        if let (Some(c), Some(h)) = (coord, hash) {
            result.insert(c.to_string(), h.to_string());
        }
    }
    Ok(result)
}

//------------------------------------------------------------------------------------------------------------
// FetchRequest: one parsed cache-aware fetch (single GET, or a POST batch)
//------------------------------------------------------------------------------------------------------------
pub struct FetchEntry {
    pub coordinate: phext::Coordinate,
    pub client_hash: Option<String>,
    pub has_content: bool,
}

pub struct FetchRequest {
    pub entries: Vec<FetchEntry>,
    pub single: bool, // GET form: answer with one object; POST form: answer with an array
}

/// GET form: c=<coord> (strict), h=<hash> (may be empty/absent), has=<0|1|true|false>
pub fn parse_fetch_get(parsed: &HashMap<String, String>) -> Result<FetchRequest, String> {
    let raw_coord = parsed.get("c").map(|s| s.trim()).filter(|s| !s.is_empty())
        .ok_or("fetch needs a coordinate (c=...)")?;
    let coordinate = crate::sq::parse_coordinate_strict(raw_coord)
        .ok_or(format!("invalid coordinate: {}", raw_coord))?;
    let client_hash = parsed.get("h").map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    let has_content = matches!(parsed.get("has").map(|s| s.trim().to_lowercase()).as_deref(),
        Some("1") | Some("true"));
    Ok(FetchRequest {
        entries: vec![FetchEntry { coordinate, client_hash, has_content }],
        single: true,
    })
}

/// POST form: body is a JSON array of {"coordinate", "hash", "has_content"}
pub fn parse_fetch_post(body: &str) -> Result<FetchRequest, String> {
    let value: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| format!("invalid fetch body: {}", e))?;
    let list = value.as_array().ok_or("fetch body must be a JSON array")?;
    if list.is_empty() {
        return Err("fetch body names no coordinates".to_string());
    }
    let mut entries = Vec::new();
    for item in list {
        let raw_coord = item.get("coordinate").and_then(|c| c.as_str())
            .ok_or("fetch entry is missing its 'coordinate'")?;
        let coordinate = crate::sq::parse_coordinate_strict(raw_coord)
            .ok_or(format!("invalid coordinate in fetch body: {}", raw_coord))?;
        let client_hash = item.get("hash").and_then(|h| h.as_str())
            .map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        let has_content = item.get("has_content").and_then(|h| h.as_bool()).unwrap_or(false);
        entries.push(FetchEntry { coordinate, client_hash, has_content });
    }
    Ok(FetchRequest { entries, single: false })
}

//------------------------------------------------------------------------------------------------------------
// fetch_one: the cache-aware decision for a single scroll
//
// Small return — {"coordinate","hash","unchanged":true} and NO content — only
// when the client holds the content and its hash is the server's current hash.
// Otherwise the normal fetch: {"coordinate","hash","unchanged":false,"scroll"}.
// A coordinate with no scroll fetches as an empty scroll at the empty hash.
//------------------------------------------------------------------------------------------------------------
fn fetch_one(phext_map: &HashMap<phext::Coordinate, String>, entry: &FetchEntry) -> serde_json::Value {
    let content = phext_map.get(&entry.coordinate).cloned().unwrap_or_default();
    let current = scroll_hash(&content);
    if entry.has_content && entry.client_hash.as_deref() == Some(current.as_str()) {
        return serde_json::json!({
            "coordinate": entry.coordinate.to_string(),
            "hash": current,
            "unchanged": true,
        });
    }
    serde_json::json!({
        "coordinate": entry.coordinate.to_string(),
        "hash": current,
        "unchanged": false,
        "scroll": content,
    })
}

/// Runs a parsed fetch request against a loaded phext map
pub fn run_fetch(phext_map: &HashMap<phext::Coordinate, String>, spec: &FetchRequest) -> String {
    if spec.single && spec.entries.len() == 1 {
        return fetch_one(phext_map, &spec.entries[0]).to_string();
    }
    let results: Vec<serde_json::Value> = spec.entries.iter()
        .map(|entry| fetch_one(phext_map, entry))
        .collect();
    serde_json::Value::Array(results).to_string()
}

//------------------------------------------------------------------------------------------------------------
// Client: a disk-backed scroll cache + the cache-aware fetch that consults it
//------------------------------------------------------------------------------------------------------------

#[derive(Clone)]
pub struct CacheEntry {
    pub hash: String,
    pub scroll: String,
}

/// Scrolls a client has already streamed in, keyed by coordinate string.
/// Persisted as one JSON file: {"<coord>": {"hash": "...", "scroll": "..."}}
pub struct ScrollCache {
    pub entries: HashMap<String, CacheEntry>,
}

impl ScrollCache {
    pub fn empty() -> ScrollCache {
        ScrollCache { entries: HashMap::new() }
    }

    /// Loads a cache file; a missing or unreadable file is simply an empty cache
    pub fn load(path: &str) -> ScrollCache {
        let mut cache = ScrollCache::empty();
        if let Ok(text) = std::fs::read_to_string(path) {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                if let Some(obj) = value.as_object() {
                    for (coord, entry) in obj {
                        let hash = entry.get("hash").and_then(|h| h.as_str());
                        let scroll = entry.get("scroll").and_then(|s| s.as_str());
                        if let (Some(h), Some(s)) = (hash, scroll) {
                            cache.entries.insert(coord.clone(), CacheEntry {
                                hash: h.to_string(),
                                scroll: s.to_string(),
                            });
                        }
                    }
                }
            }
        }
        cache
    }

    pub fn save(&self, path: &str) -> Result<(), String> {
        let obj: serde_json::Map<String, serde_json::Value> = self.entries.iter()
            .map(|(coord, entry)| (coord.clone(), serde_json::json!({
                "hash": entry.hash,
                "scroll": entry.scroll,
            })))
            .collect();
        let text = serde_json::Value::Object(obj).to_string();
        std::fs::write(path, text).map_err(|e| format!("unable to write cache {}: {}", path, e))
    }

    pub fn get(&self, coordinate: &str) -> Option<&CacheEntry> {
        self.entries.get(coordinate)
    }

    pub fn put(&mut self, coordinate: &str, hash: String, scroll: String) {
        self.entries.insert(coordinate.to_string(), CacheEntry { hash, scroll });
    }
}

//------------------------------------------------------------------------------------------------------------
// minimal HTTP client (std TcpStream only — SQ hand-rolls its HTTP server the
// same way; no new dependency for the client half)
//------------------------------------------------------------------------------------------------------------
fn http_request(base: &str, method: &str, path: &str, body: Option<&str>, key: &Option<String>) -> Result<String, String> {
    let authority = base.trim_start_matches("http://").trim_start_matches("https://")
        .trim_end_matches('/');
    let mut stream = TcpStream::connect(authority)
        .map_err(|e| format!("unable to connect to {}: {}", authority, e))?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(15)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(15)));

    let mut request = format!("{} {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n", method, path, authority);
    if let Some(k) = key {
        request += &format!("X-SQ-API-Key: {}\r\n", k);
    }
    if let Some(b) = body {
        request += &format!("Content-Type: application/json\r\nContent-Length: {}\r\n", b.len());
    }
    request += "\r\n";
    if let Some(b) = body {
        request += b;
    }
    stream.write_all(request.as_bytes())
        .map_err(|e| format!("unable to send request: {}", e))?;

    let mut raw = Vec::new();
    stream.read_to_end(&mut raw)
        .map_err(|e| format!("unable to read response: {}", e))?;
    let text = String::from_utf8_lossy(&raw).to_string();
    let status_line = text.lines().next().unwrap_or("").to_string();
    if !status_line.contains(" 200 ") {
        return Err(format!("server answered: {}", status_line));
    }
    match text.find("\r\n\r\n") {
        Some(pos) => Ok(text[pos + 4..].to_string()),
        None => Err("malformed HTTP response".to_string()),
    }
}

pub fn http_get(base: &str, path: &str, key: &Option<String>) -> Result<String, String> {
    http_request(base, "GET", path, None, key)
}

/// Downloads the seed envelope for a phext and parses it into coordinate -> hash
pub fn fetch_seed(base: &str, phext_name: &str, key: &Option<String>) -> Result<HashMap<String, String>, String> {
    let body = http_get(base, &format!("/api/v2/seed?p={}", phext_name), key)?;
    parse_seed(&body)
}

//------------------------------------------------------------------------------------------------------------
// fetch_cached: the cache-aware fetch, end to end
//
//   1. Consult the seed. If the cache already holds this coordinate at the
//      seed's hash, serve it locally — no content request is made at all.
//   2. Otherwise ask /api/v2/fetch, presenting the cached hash (if any) and
//      whether the content is held. A server "unchanged" answer (the seed was
//      raced by an edit) is served from the cache after all.
//   3. A full answer is verified against its stated hash before it enters
//      the cache — a streamed scroll is never trusted on arrival alone.
//
// Returns (content, served_without_content_transfer).
//------------------------------------------------------------------------------------------------------------
pub fn fetch_cached(base: &str, phext_name: &str, key: &Option<String>,
                    coordinate: &str, cache: &mut ScrollCache) -> Result<(String, bool), String> {
    let seed = fetch_seed(base, phext_name, key)?;
    let seed_hash = seed.get(coordinate);

    if let Some(entry) = cache.get(coordinate) {
        if seed_hash == Some(&entry.hash) {
            return Ok((entry.scroll.clone(), true));
        }
    }

    let (client_hash, has_content) = match cache.get(coordinate) {
        Some(entry) => (entry.hash.clone(), true),
        None => (String::new(), false),
    };
    let path = format!("/api/v2/fetch?p={}&c={}&h={}&has={}",
        phext_name, coordinate, client_hash, if has_content { 1 } else { 0 });
    let body = http_get(base, &path, key)?;
    let value: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("invalid fetch response: {}", e))?;

    let unchanged = value.get("unchanged").and_then(|u| u.as_bool()).unwrap_or(false);
    let hash = value.get("hash").and_then(|h| h.as_str()).unwrap_or("").to_string();

    if unchanged {
        // The server confirms our cached copy is current (e.g. the seed raced
        // an edit that has since reverted). Serve the cache; keep its hash in
        // step with the server's confirmation.
        if let Some(entry) = cache.get(coordinate) {
            let content = entry.scroll.clone();
            if entry.hash != hash {
                cache.put(coordinate, hash, content.clone());
            }
            return Ok((content, true));
        }
        // "unchanged" for content we do not hold should never happen; fall
        // through to one unconditional fetch rather than inventing content.
        let retry = http_get(base, &format!("/api/v2/fetch?p={}&c={}&has=0", phext_name, coordinate), key)?;
        let value: serde_json::Value = serde_json::from_str(&retry)
            .map_err(|e| format!("invalid fetch response: {}", e))?;
        let content = value.get("scroll").and_then(|s| s.as_str()).unwrap_or("").to_string();
        let hash = value.get("hash").and_then(|h| h.as_str()).unwrap_or("").to_string();
        if scroll_hash(&content) != hash {
            return Err(format!("hash mismatch for {}: content does not match its stated hash", coordinate));
        }
        cache.put(coordinate, hash, content.clone());
        return Ok((content, false));
    }

    let content = value.get("scroll").and_then(|s| s.as_str()).unwrap_or("").to_string();
    if scroll_hash(&content) != hash {
        return Err(format!("hash mismatch for {}: content does not match its stated hash", coordinate));
    }
    cache.put(coordinate, hash, content.clone());
    Ok((content, false))
}
