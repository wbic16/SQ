//------------------------------------------------------------------------------------------------------------
// file: tests.rs
// purpose: Provides a battery of unit tests to improve project quality.
//
// note: You can run these tests with `cargo test`.
//------------------------------------------------------------------------------------------------------------

#[cfg(test)]
use libphext::phext;

#[test]
fn test_insert() {
  let mut scroll = String::new();
  let command = "insert".to_string();
  let buffer = String::new();
  let coordinate = phext::to_coordinate("1.1.1/1.1.1/1.1.2");
  let update = "Hello World!".to_string();
  let filename = "insert.phext".to_string();
  let mut map = phext::explode(&buffer);
  let done = crate::sq::process(1, "memory".to_string(), &mut scroll, command, &mut map, coordinate, update, filename, crate::HashAlgorithm::Xor, 100);
  let buffer = phext::implode(map);

  assert_eq!(buffer, "\x17Hello World!");
  assert_eq!(done, false);
}

#[test]
fn test_select() {
  let mut scroll = String::new();
  let command = "select".to_string();
  let buffer = "\x17\x17Third Scroll Content".to_string();
  let coordinate = phext::to_coordinate("1.1.1/1.1.1/1.1.3");
  let update = "ignored text".to_string();
  let filename = "select.phext".to_string();
  let mut map = phext::explode(&buffer);
  let done = crate::sq::process(1, "memory".to_string(), &mut scroll, command, &mut map, coordinate, update, filename, crate::HashAlgorithm::Xor, 100);

  assert_eq!(buffer, "\x17\x17Third Scroll Content");
  assert_eq!(scroll, "Third Scroll Content");
  assert_eq!(done, false);
}

#[test]
fn test_update() {
  let mut scroll = String::new();
  let command = "update".to_string();
  let buffer = "\x17\x18\x17Third Scroll Original".to_string();
  let coordinate = phext::to_coordinate("1.1.1/1.1.1/1.2.2");
  let update = "Full Rewrite at 1.2.2".to_string();
  let filename = "update.phext".to_string();
  let mut map = phext::explode(&buffer);
  let done = crate::sq::process(1, "memory".to_string(), &mut scroll, command, &mut map, coordinate, update, filename, crate::HashAlgorithm::Xor, 100);
  let buffer = phext::implode(map);

  assert_eq!(buffer, "\x18\x17Full Rewrite at 1.2.2");
  assert_eq!(scroll, "Updated 21 bytes");
  assert_eq!(done, false);
}

#[test]
fn test_delete() {
  let mut scroll = String::new();
  let command = "delete".to_string();
  let buffer = "\x17\x18\x17Third Scroll Original".to_string();
  let coordinate = phext::to_coordinate("1.1.1/1.1.1/1.2.2");
  let update = "".to_string();
  let filename = "delete.phext".to_string();
  let mut map = phext::explode(&buffer);
  let done = crate::sq::process(1, "memory".to_string(), &mut scroll, command, &mut map, coordinate, update, filename, crate::HashAlgorithm::Xor, 100);
  let buffer = phext::implode(map);

  assert_eq!(buffer, "");
  assert_eq!(scroll, "Removed 21 bytes");
  assert_eq!(done, false);
}

#[test]
fn test_save() {
  let mut scroll = String::new();
  let command = "save".to_string();
  let buffer = "\x17\x18\x17Save Test".to_string();
  let coordinate = phext::to_coordinate("1.1.1/1.1.1/1.2.2");
  let update = "Save Test at 1.2.2".to_string();
  let filename = "save.phext".to_string();
  let mut map = phext::explode(&buffer);
  let done = crate::sq::process(1, "memory".to_string(), &mut scroll, command, &mut map, coordinate, update, filename, crate::HashAlgorithm::Xor, 100);
  let buffer = phext::implode(map);

  assert_eq!(buffer, "\x18\x17Save Test");
  assert_eq!(scroll, "Wrote 11 bytes to save.phext");
  assert_eq!(done, false);

  std::fs::remove_file("save.phext").expect("Unable to find save.phext");
}

#[test]
fn test_toc() {
  let scroll = "hello\x17from\x18beyond\x19the\x1astars\x1cnot\x1dan\x1eevil\x1ffuzzle\x01just a warm fuzzy.";
  let toc = phext::textmap(scroll);
  assert_eq!(toc, "* 1.1.1/1.1.1/1.1.1: hello
* 1.1.1/1.1.1/1.1.2: from
* 1.1.1/1.1.1/1.2.1: beyond
* 1.1.1/1.1.1/2.1.1: the
* 1.1.1/1.1.2/1.1.1: stars
* 1.1.1/1.2.1/1.1.1: not
* 1.1.1/2.1.1/1.1.1: an
* 1.1.2/1.1.1/1.1.1: evil
* 1.2.1/1.1.1/1.1.1: fuzzle
* 2.1.1/1.1.1/1.1.1: just a warm fuzzy.
");
}

#[test]
fn test_auth_valid_bearer() {
  let key = Some("pmb-v1-abc123".to_string());
  let header = "GET /api/v2/version HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer pmb-v1-abc123\r\n\r\n";
  assert_eq!(crate::validate_auth(header, &key), true);
}

#[test]
fn test_auth_invalid_key() {
  let key = Some("pmb-v1-abc123".to_string());
  let header = "GET /api/v2/version HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer pmb-v1-wrong\r\n\r\n";
  assert_eq!(crate::validate_auth(header, &key), false);
}

#[test]
fn test_auth_missing_header() {
  let key = Some("pmb-v1-abc123".to_string());
  let header = "GET /api/v2/version HTTP/1.1\r\nHost: localhost\r\n\r\n";
  assert_eq!(crate::validate_auth(header, &key), false);
}

#[test]
fn test_auth_disabled() {
  let key: Option<String> = None;
  let header = "GET /api/v2/version HTTP/1.1\r\nHost: localhost\r\n\r\n";
  assert_eq!(crate::validate_auth(header, &key), true);
}

#[test]
fn test_tenant_path_valid() {
  let dir = Some("/tmp/tenant".to_string());
  let result = crate::validate_tenant_path("mydata", &dir);
  assert_eq!(result, Some("/tmp/tenant/mydata.phext".to_string()));
}

#[test]
fn test_tenant_path_traversal_blocked() {
  let dir = Some("/tmp/tenant".to_string());
  assert_eq!(crate::validate_tenant_path("../../etc/passwd", &dir), None);
  assert_eq!(crate::validate_tenant_path("sub/path", &dir), None);
  assert_eq!(crate::validate_tenant_path("back\\slash", &dir), None);
}

#[test]
fn test_tenant_path_no_restriction() {
  let dir: Option<String> = None;
  let result = crate::validate_tenant_path("mydata", &dir);
  assert_eq!(result, Some("mydata.phext".to_string()));
}

#[test]
fn convert_from_json() {
  // { "field": "value", "field2": "value 2" }
}

#[test]
fn convert_from_xml() {
  // <tag a1="1" a2="2">value</tag>
  // <group>
  //   <tag a1="alpha" b1="beta" />
  //   <tag a1="gamma" b1="delta">epsilon</tag>
  // </group> 
}

#[test]
fn test_exit() {
  let mut scroll = String::new();
  let command = "shutdown".to_string();
  let mut buffer = phext::explode("");
  let coordinate = phext::to_coordinate("1.1.1/1.1.1/1.1.1");
  let update = "Shutdown Test".to_string();
  let filename = "shutdown.phext".to_string();

  let done = crate::sq::process(1, "memory".to_string(), &mut scroll, command, &mut buffer, coordinate, update, filename, crate::HashAlgorithm::Xor, 100);

  assert_eq!(done, true);
}

// =========================================================================================================
// implode_ref correctness tests
// =========================================================================================================

// Verify implode_ref produces the same output as phext::implode for a known phext
#[test]
fn test_implode_ref_matches_implode_simple() {
    let raw = "hello\x17from\x18beyond\x19the\x1astars";
    let map = phext::explode(raw);
    let expected = phext::implode(map.clone());
    let actual = crate::sq::implode_ref(&map);
    assert_eq!(actual, expected,
        "implode_ref must produce identical bytes to phext::implode");
}

// Verify implode_ref handles all 9 delimiter levels
#[test]
fn test_implode_ref_all_delimiters() {
    let raw = "hello\x17from\x18beyond\x19the\x1astars\x1cnot\x1dan\x1eevil\x1ffuzzle\x01just a warm fuzzy.";
    let map = phext::explode(raw);
    let expected = phext::implode(map.clone());
    let actual = crate::sq::implode_ref(&map);
    assert_eq!(actual, expected,
        "implode_ref must handle all 9 delimiter levels correctly");
}

// Verify implode_ref on a single scroll at origin
#[test]
fn test_implode_ref_single_scroll_at_origin() {
    let raw = "just one scroll";
    let map = phext::explode(raw);
    let expected = phext::implode(map.clone());
    let actual = crate::sq::implode_ref(&map);
    assert_eq!(actual, expected);
}

// Verify implode_ref on a single scroll NOT at origin (requires leading delimiters)
#[test]
fn test_implode_ref_single_scroll_offset() {
    let raw = "\x17Hello World!";
    let map = phext::explode(raw);
    let expected = phext::implode(map.clone());
    let actual = crate::sq::implode_ref(&map);
    assert_eq!(actual, expected);
}

// Verify implode_ref on empty map
#[test]
fn test_implode_ref_empty() {
    let map = phext::explode("");
    let expected = phext::implode(map.clone());
    let actual = crate::sq::implode_ref(&map);
    assert_eq!(actual, expected);
}

// Verify implode_ref round-trips correctly through explode
#[test]
fn test_implode_ref_roundtrip() {
    let raw = "\x18\x17Full Rewrite at 1.2.2";
    let map = phext::explode(raw);
    let serialized = crate::sq::implode_ref(&map);
    let re_exploded = phext::explode(&serialized);

    // Every non-empty scroll should survive the round-trip
    for (coord, content) in &map {
        if !content.is_empty() {
            assert_eq!(re_exploded.get(coord).map(|s| s.as_str()), Some(content.as_str()),
                "Non-empty scroll at {} must survive round-trip", coord);
        }
    }
}

// Verify implode_ref with a multi-scroll gap (e.g., scroll 1 and scroll 5)
#[test]
fn test_implode_ref_with_gaps() {
    use std::collections::HashMap;
    let mut map: HashMap<phext::Coordinate, String> = HashMap::new();
    map.insert(phext::to_coordinate("1.1.1/1.1.1/1.1.1"), "first".to_string());
    map.insert(phext::to_coordinate("1.1.1/1.1.1/1.1.5"), "fifth".to_string());

    let expected = phext::implode(map.clone());
    let actual = crate::sq::implode_ref(&map);
    assert_eq!(actual, expected,
        "implode_ref must produce correct delimiters across gaps");
}

// Verify implode_ref with section + scroll gap
#[test]
fn test_implode_ref_section_gap() {
    use std::collections::HashMap;
    let mut map: HashMap<phext::Coordinate, String> = HashMap::new();
    map.insert(phext::to_coordinate("1.1.1/1.1.1/1.1.1"), "origin".to_string());
    map.insert(phext::to_coordinate("1.1.1/1.1.1/1.3.4"), "deep".to_string());

    let expected = phext::implode(map.clone());
    let actual = crate::sq::implode_ref(&map);
    assert_eq!(actual, expected);
}

// Verify implode_ref across a high-level break (different series)
#[test]
fn test_implode_ref_cross_series() {
    use std::collections::HashMap;
    let mut map: HashMap<phext::Coordinate, String> = HashMap::new();
    map.insert(phext::to_coordinate("1.1.1/1.1.1/1.1.1"), "alpha".to_string());
    map.insert(phext::to_coordinate("1.1.2/1.1.1/1.1.1"), "beta".to_string());

    let expected = phext::implode(map.clone());
    let actual = crate::sq::implode_ref(&map);
    assert_eq!(actual, expected);
}

// =========================================================================================================
// Memory pressure tests
// =========================================================================================================

// Verify that implode_ref doesn't clone the map contents
// (We can't directly measure allocations without an allocator hook, but we can
//  verify it works on a map large enough that a clone would be conspicuous.)
#[test]
fn test_implode_ref_large_map_no_panic() {
    use std::collections::HashMap;
    let mut map: HashMap<phext::Coordinate, String> = HashMap::new();

    // Build a map with 1000 scrolls, each 1 KB — total ~1 MB of content.
    // phext::implode(map.clone()) would momentarily hold 2 MB + serialized output.
    // implode_ref(&map) should hold only the output.
    let payload = "x".repeat(1024);
    for i in 1..=1000 {
        let section = ((i - 1) / 100) + 1;
        let scroll = ((i - 1) % 100) + 1;
        let coord_str = format!("1.1.1/1.1.1/{}.{}.1", section, scroll);
        map.insert(phext::to_coordinate(&coord_str), payload.clone());
    }

    let result = crate::sq::implode_ref(&map);
    assert!(result.len() > 1_000_000, "Serialized output should be > 1 MB");

    // Verify round-trip integrity
    let re_exploded = phext::explode(&result);
    let non_empty_original: usize = map.values().filter(|v| !v.is_empty()).count();
    let non_empty_roundtrip: usize = re_exploded.values().filter(|v| !v.is_empty()).count();
    assert_eq!(non_empty_original, non_empty_roundtrip,
        "All 1000 non-empty scrolls must survive round-trip");
}

// Verify that insert + implode_ref + explode round-trips correctly under churn
#[test]
fn test_churn_insert_delete_implode_ref() {
    use std::collections::HashMap;
    let mut map: HashMap<phext::Coordinate, String> = HashMap::new();

    // Insert 100 scrolls
    for i in 1..=100 {
        let coord = phext::to_coordinate(&format!("1.1.1/1.1.1/1.1.{}", i));
        map.insert(coord, format!("scroll-{}", i));
    }

    // Delete every other scroll
    for i in (1..=100).step_by(2) {
        let coord = phext::to_coordinate(&format!("1.1.1/1.1.1/1.1.{}", i));
        map.remove(&coord);
    }

    // Verify implode_ref matches implode
    let expected = phext::implode(map.clone());
    let actual = crate::sq::implode_ref(&map);
    assert_eq!(actual, expected,
        "implode_ref must match after insert+delete churn");
}

// Verify the connection counter tracks correctly
#[test]
fn test_active_connection_counter() {
    use std::sync::atomic::Ordering;

    let before = crate::ACTIVE_CONNECTIONS.load(Ordering::Relaxed);
    crate::ACTIVE_CONNECTIONS.fetch_add(1, Ordering::Relaxed);
    crate::ACTIVE_CONNECTIONS.fetch_add(1, Ordering::Relaxed);
    let during = crate::ACTIVE_CONNECTIONS.load(Ordering::Relaxed);
    assert_eq!(during, before + 2);

    crate::ACTIVE_CONNECTIONS.fetch_sub(1, Ordering::Relaxed);
    crate::ACTIVE_CONNECTIONS.fetch_sub(1, Ordering::Relaxed);
    let after = crate::ACTIVE_CONNECTIONS.load(Ordering::Relaxed);
    assert_eq!(after, before, "Counter must return to baseline after sub");
}

// Verify MAX_BODY_SIZE is sane
#[test]
fn test_max_body_size_sane() {
    assert!(crate::MAX_BODY_SIZE <= crate::MAX_BUFFER_SIZE,
        "MAX_BODY_SIZE ({}) should not exceed MAX_BUFFER_SIZE ({})",
        crate::MAX_BODY_SIZE, crate::MAX_BUFFER_SIZE);
    assert!(crate::MAX_BODY_SIZE >= 1024 * 1024,
        "MAX_BODY_SIZE should be at least 1 MB for practical use");
}

// Verify that status command uses implode_ref (no clone) by checking output format
#[test]
fn test_status_output_format() {
    use std::collections::HashMap;
    let mut map: HashMap<phext::Coordinate, String> = HashMap::new();
    map.insert(phext::to_coordinate("1.1.1/1.1.1/1.1.1"), "hello".to_string());
    map.insert(phext::to_coordinate("1.1.1/1.1.1/1.1.2"), "world".to_string());

    let mut scroll = String::new();
    crate::sq::process(
        42, "test.phext".to_string(), &mut scroll, "status".to_string(),
        &mut map, phext::to_coordinate("1.1.1/1.1.1/1.1.1"),
        String::new(), String::new(), crate::HashAlgorithm::Xor, 100,
    );

    assert!(scroll.contains("Connection ID: 42"));
    assert!(scroll.contains("Hosting: test.phext"));
    assert!(scroll.contains("Phext Size:"));
    assert!(scroll.contains("Scrolls:"));
}

// Stress test: many small mutations followed by implode_ref serialization
#[test]
fn test_mutation_flush_pressure() {
    use std::collections::HashMap;
    let mut map: HashMap<phext::Coordinate, String> = HashMap::new();

    // Simulate 500 rapid insert-then-serialize cycles (mimics the hot path)
    for i in 1..=500 {
        let section = ((i - 1) / 50) + 1;
        let scroll_num = ((i - 1) % 50) + 1;
        let coord = phext::to_coordinate(&format!("1.1.1/1.1.1/{}.{}.1", section, scroll_num));
        map.insert(coord, format!("payload-{}", i));

        // This is what the mutation flush does on every write
        let _serialized = crate::sq::implode_ref(&map);
    }

    // Final consistency check
    let final_ref = crate::sq::implode_ref(&map);
    let final_clone = phext::implode(map.clone());
    assert_eq!(final_ref, final_clone,
        "After 500 mutations, implode_ref must still match implode");
}

// -----------------------------------------------------------------------------------------------------------
// Coordinate Query API (v0.6.0)
// -----------------------------------------------------------------------------------------------------------
#[cfg(test)]
fn query_fixture() -> std::collections::HashMap<phext::Coordinate, String> {
  let mut map = std::collections::HashMap::new();
  for (c, s) in [
    ("1.1.1/1.1.1/1.1.1", "origin"),
    ("1.1.1/1.1.1/1.1.2", "second"),
    ("1.1.1/1.1.1/1.2.1", "next section"),
    ("1.1.1/1.1.2/1.1.1", "next book"),
    ("2.1.1/1.1.1/1.1.1", "next library"),
    ("1.1.1/1.1.1/1.1.3", ""),
  ] { map.insert(phext::to_coordinate(c), s.to_string()); }
  map
}

#[cfg(test)]
fn q(from: &str, to: &str, set: &str, limit: &str) -> Result<crate::sq::QuerySpec, String> {
  let (f, t, s, l) = (from.to_string(), to.to_string(), set.to_string(), limit.to_string());
  crate::sq::parse_query(Some(&f), Some(&t), Some(&s), Some(&l))
}

#[test]
fn test_query_range_inclusive_and_sorted() {
  let map = query_fixture();
  let out = crate::sq::run_query(&map, &q("1.1.1/1.1.1/1.1.1", "1.1.1/1.1.2/1.1.1", "", "").unwrap());
  let v: serde_json::Value = serde_json::from_str(&out).unwrap();
  let coords: Vec<&str> = v.as_array().unwrap().iter().map(|r| r["coordinate"].as_str().unwrap()).collect();
  assert_eq!(coords, vec!["1.1.1/1.1.1/1.1.1", "1.1.1/1.1.1/1.1.2", "1.1.1/1.1.1/1.2.1", "1.1.1/1.1.2/1.1.1"]);
}

#[test]
fn test_query_range_skips_empty_scrolls() {
  let map = query_fixture();
  let out = crate::sq::run_query(&map, &q("1.1.1/1.1.1/1.1.3", "1.1.1/1.1.1/1.1.3", "", "").unwrap());
  assert_eq!(out, "[]");
}

#[test]
fn test_query_set_keeps_missing_coordinates() {
  let map = query_fixture();
  let out = crate::sq::run_query(&map, &q("", "", "2.1.1/1.1.1/1.1.1, 9.9.9/9.9.9/9.9.9,1.1.1/1.1.1/1.1.1", "").unwrap());
  let v: serde_json::Value = serde_json::from_str(&out).unwrap();
  let rows = v.as_array().unwrap();
  assert_eq!(rows.len(), 3);
  assert_eq!(rows[0]["coordinate"], "1.1.1/1.1.1/1.1.1");
  assert_eq!(rows[1]["scroll"], "next library");
  assert_eq!(rows[2]["coordinate"], "9.9.9/9.9.9/9.9.9");
  assert_eq!(rows[2]["scroll"], "");
}

#[test]
fn test_query_range_and_set_union_dedups() {
  let map = query_fixture();
  let out = crate::sq::run_query(&map, &q("1.1.1/1.1.1/1.1.1", "1.1.1/1.1.1/1.1.2", "1.1.1/1.1.1/1.1.2,2.1.1/1.1.1/1.1.1", "").unwrap());
  let v: serde_json::Value = serde_json::from_str(&out).unwrap();
  assert_eq!(v.as_array().unwrap().len(), 3);
}

#[test]
fn test_query_limit() {
  let map = query_fixture();
  let out = crate::sq::run_query(&map, &q("1.1.1/1.1.1/1.1.1", "9.9.9/9.9.9/9.9.9", "", "2").unwrap());
  let v: serde_json::Value = serde_json::from_str(&out).unwrap();
  assert_eq!(v.as_array().unwrap().len(), 2);
}

#[test]
fn test_query_escapes_phext_delimiters_and_quotes() {
  let mut map = std::collections::HashMap::new();
  map.insert(phext::to_coordinate("1.1.1/1.1.1/1.1.1"), "say \"hi\"\\ \x17 done".to_string());
  let out = crate::sq::run_query(&map, &q("", "", "1.1.1/1.1.1/1.1.1", "").unwrap());
  let v: serde_json::Value = serde_json::from_str(&out).unwrap();
  assert_eq!(v[0]["scroll"], "say \"hi\"\\ \x17 done");
}

#[test]
fn test_query_rejects_bad_input() {
  assert!(q("", "", "", "").is_err());
  assert!(q("1.1.1/1.1.1/1.1.1", "", "", "").is_err());
  assert!(q("1.1.1/1.1.1/1.1.2", "1.1.1/1.1.1/1.1.1", "", "").is_err());
  assert!(q("", "", "1.1.1/1.1.1", "").is_err());
  assert!(q("", "", "0.1.1/1.1.1/1.1.1", "").is_err());
  assert!(q("", "", "a.b.c/1.1.1/1.1.1", "").is_err());
  assert!(q("", "", "1.1.1/1.1.1/1.1.1", "many").is_err());
}

//------------------------------------------------------------------------------------------------------------
// Seed cache + cache-aware fetch tests
//
// Hash function: xxh3_128 via libphext's phext::checksum (Will, 2026-11-10) —
// the same per-scroll hash the delta command diffs with, as 32 lowercase hex.
//------------------------------------------------------------------------------------------------------------

#[cfg(test)]
fn seed_fixture() -> std::collections::HashMap<phext::Coordinate, String> {
  let mut map = std::collections::HashMap::new();
  map.insert(phext::to_coordinate("1.1.1/1.1.1/1.1.1"), "Hello World!".to_string());
  map.insert(phext::to_coordinate("1.1.1/1.1.1/1.1.2"), "Second scroll".to_string());
  map.insert(phext::to_coordinate("1.1.1/1.1.2/1.1.1"), "Section two opener".to_string());
  map
}

#[test]
fn test_seed_hash_is_xxh3_128_known_answer() {
  // pinned known answers for libphext's checksum (xxh3_128, hex of the digest)
  assert_eq!(crate::seed::scroll_hash(""), "99aa06d3014798d86001c324468d497f");
  assert_eq!(crate::seed::scroll_hash("Hello World!"), "bbce2257f0cec895f56f7a348bed5898");
  assert_eq!(crate::seed::scroll_hash("Hello World!"), phext::checksum("Hello World!"));
  assert_eq!(crate::seed::scroll_hash("Hello World!").len(), 32);
}

#[test]
fn test_seed_envelope_lists_every_populated_scroll() {
  let map = seed_fixture();
  let out = crate::seed::build_seed(&map);
  let v: serde_json::Value = serde_json::from_str(&out).unwrap();
  assert_eq!(v["algorithm"], "xxh3_128");
  assert_eq!(v["scrolls"], 3);
  let hashes = v["hashes"].as_array().unwrap();
  assert_eq!(hashes.len(), 3);
  // sorted by coordinate, library down to scroll
  assert_eq!(hashes[0]["coordinate"], "1.1.1/1.1.1/1.1.1");
  assert_eq!(hashes[1]["coordinate"], "1.1.1/1.1.1/1.1.2");
  assert_eq!(hashes[2]["coordinate"], "1.1.1/1.1.2/1.1.1");
  // every hash is the xxh3_128 of the actual scroll content
  for row in hashes {
    let coord = row["coordinate"].as_str().unwrap();
    let content = map.get(&phext::to_coordinate(coord)).unwrap();
    assert_eq!(row["hash"], crate::seed::scroll_hash(content));
    assert_eq!(row["hash"], phext::checksum(content));
  }
  // the envelope carries hashes only — no scroll content leaks into it
  assert!(!out.contains("Hello World!"));
  assert!(!out.contains("Second scroll"));
}

#[test]
fn test_seed_envelope_empty_phext() {
  let map: std::collections::HashMap<phext::Coordinate, String> = std::collections::HashMap::new();
  let out = crate::seed::build_seed(&map);
  let v: serde_json::Value = serde_json::from_str(&out).unwrap();
  assert_eq!(v["algorithm"], "xxh3_128");
  assert_eq!(v["scrolls"], 0);
  assert_eq!(v["hashes"].as_array().unwrap().len(), 0);
}

#[test]
fn test_seed_envelope_skips_empty_scrolls() {
  let mut map = seed_fixture();
  map.insert(phext::to_coordinate("1.1.1/1.1.1/1.1.3"), String::new());
  let v: serde_json::Value = serde_json::from_str(&crate::seed::build_seed(&map)).unwrap();
  assert_eq!(v["scrolls"], 3);
}

#[test]
fn test_seed_process_command_matches_build_seed() {
  let mut map = seed_fixture();
  let mut scroll = String::new();
  let done = crate::sq::process(1, "memory".to_string(), &mut scroll, "seed".to_string(),
    &mut map, phext::to_coordinate("1.1.1/1.1.1/1.1.1"), String::new(),
    "seed.phext".to_string(), crate::HashAlgorithm::Xor, 100);
  assert_eq!(done, false);
  assert_eq!(scroll, crate::seed::build_seed(&seed_fixture()));
}

#[test]
fn test_seed_parse_roundtrip() {
  let map = seed_fixture();
  let seed = crate::seed::parse_seed(&crate::seed::build_seed(&map)).unwrap();
  assert_eq!(seed.len(), 3);
  assert_eq!(seed["1.1.1/1.1.1/1.1.1"], crate::seed::scroll_hash("Hello World!"));
  assert_eq!(seed["1.1.1/1.1.2/1.1.1"], crate::seed::scroll_hash("Section two opener"));
}

#[cfg(test)]
fn fetch_get(coord: &str, hash: &str, has: &str) -> crate::seed::FetchRequest {
  let mut parsed = std::collections::HashMap::new();
  parsed.insert("c".to_string(), coord.to_string());
  parsed.insert("h".to_string(), hash.to_string());
  parsed.insert("has".to_string(), has.to_string());
  crate::seed::parse_fetch_get(&parsed).unwrap()
}

#[test]
fn test_fetch_unchanged_is_small_and_content_free() {
  // branch (a): has content + hash matches -> small confirmation, NO content bytes
  let map = seed_fixture();
  let hash = crate::seed::scroll_hash("Hello World!");
  let out = crate::seed::run_fetch(&map, &fetch_get("1.1.1/1.1.1/1.1.1", &hash, "1"));
  let v: serde_json::Value = serde_json::from_str(&out).unwrap();
  assert_eq!(v["unchanged"], true);
  assert_eq!(v["coordinate"], "1.1.1/1.1.1/1.1.1");
  assert_eq!(v["hash"], hash);
  assert!(v.get("scroll").is_none());
  assert!(!out.contains("Hello World!"));
  println!("unchanged confirmation: {} bytes: {}", out.len(), out);
  assert!(out.len() < 120, "unchanged confirmation grew to {} bytes", out.len());
}

#[test]
fn test_fetch_without_content_returns_full_scroll_even_at_matching_hash() {
  // branch (b): has_content=false -> full content, even when the hash matches
  let map = seed_fixture();
  let hash = crate::seed::scroll_hash("Second scroll");
  let out = crate::seed::run_fetch(&map, &fetch_get("1.1.1/1.1.1/1.1.2", &hash, "0"));
  let v: serde_json::Value = serde_json::from_str(&out).unwrap();
  assert_eq!(v["unchanged"], false);
  assert_eq!(v["scroll"], "Second scroll");
  assert_eq!(v["hash"], hash);
}

#[test]
fn test_fetch_stale_hash_returns_full_scroll_and_current_hash() {
  // branch (c): stale hash -> full content + the new current hash
  let map = seed_fixture();
  let out = crate::seed::run_fetch(&map, &fetch_get("1.1.1/1.1.1/1.1.1", "00000000000000000000000000000000", "1"));
  let v: serde_json::Value = serde_json::from_str(&out).unwrap();
  assert_eq!(v["unchanged"], false);
  assert_eq!(v["scroll"], "Hello World!");
  assert_eq!(v["hash"], crate::seed::scroll_hash("Hello World!"));
}

#[test]
fn test_fetch_missing_coordinate_returns_empty_scroll() {
  let map = seed_fixture();
  let out = crate::seed::run_fetch(&map, &fetch_get("9.9.9/9.9.9/9.9.9", "", "0"));
  let v: serde_json::Value = serde_json::from_str(&out).unwrap();
  assert_eq!(v["unchanged"], false);
  assert_eq!(v["scroll"], "");
  assert_eq!(v["hash"], crate::seed::scroll_hash(""));
}

#[test]
fn test_fetch_batch_applies_the_logic_per_entry() {
  let map = seed_fixture();
  let body = format!(r#"[
    {{"coordinate": "1.1.1/1.1.1/1.1.1", "hash": "{}", "has_content": true}},
    {{"coordinate": "1.1.1/1.1.1/1.1.2", "hash": "{}", "has_content": false}},
    {{"coordinate": "1.1.1/1.1.2/1.1.1", "hash": "deadbeef", "has_content": true}}
  ]"#, crate::seed::scroll_hash("Hello World!"), crate::seed::scroll_hash("Second scroll"));
  let spec = crate::seed::parse_fetch_post(&body).unwrap();
  let out = crate::seed::run_fetch(&map, &spec);
  let v: serde_json::Value = serde_json::from_str(&out).unwrap();
  let rows = v.as_array().unwrap();
  assert_eq!(rows.len(), 3);
  assert_eq!(rows[0]["unchanged"], true);
  assert!(rows[0].get("scroll").is_none());
  assert_eq!(rows[1]["unchanged"], false);
  assert_eq!(rows[1]["scroll"], "Second scroll");
  assert_eq!(rows[2]["unchanged"], false);
  assert_eq!(rows[2]["scroll"], "Section two opener");
  assert_eq!(rows[2]["hash"], crate::seed::scroll_hash("Section two opener"));
}

#[test]
fn test_fetch_rejects_bad_input() {
  let mut parsed = std::collections::HashMap::new();
  parsed.insert("c".to_string(), "1.1.1/1.1.1".to_string());
  assert!(crate::seed::parse_fetch_get(&parsed).is_err());
  let mut parsed = std::collections::HashMap::new();
  parsed.insert("c".to_string(), "0.1.1/1.1.1/1.1.1".to_string());
  assert!(crate::seed::parse_fetch_get(&parsed).is_err());
  assert!(crate::seed::parse_fetch_post("not json").is_err());
  assert!(crate::seed::parse_fetch_post("[]").is_err());
  assert!(crate::seed::parse_fetch_post(r#"[{"hash": "x"}]"#).is_err());
  assert!(crate::seed::parse_fetch_post(r#"[{"coordinate": "1.1.1/1.1.1"}]"#).is_err());
}

#[test]
fn test_scroll_cache_persistence_roundtrip() {
  let path = std::env::temp_dir().join(format!("sq-seed-cache-persist-{}.json", std::process::id()));
  let file = path.to_string_lossy().to_string();
  let _ = std::fs::remove_file(&file);
  let mut cache = crate::seed::ScrollCache::load(&file);
  assert!(cache.get("1.1.1/1.1.1/1.1.1").is_none());
  cache.put("1.1.1/1.1.1/1.1.1", crate::seed::scroll_hash("Hello World!"), "Hello World!".to_string());
  cache.save(&file).unwrap();
  let reloaded = crate::seed::ScrollCache::load(&file);
  let entry = reloaded.get("1.1.1/1.1.1/1.1.1").unwrap();
  assert_eq!(entry.scroll, "Hello World!");
  assert_eq!(entry.hash, crate::seed::scroll_hash("Hello World!"));
  let _ = std::fs::remove_file(&file);
}

// A minimal in-process SQ HTTP stand-in for the client tests: serves
// /api/v2/seed and /api/v2/fetch from a shared map using the real seed/fetch
// logic, and records the first line of every request it answers — so the
// client test can prove no content request happened by counting, not by trust.
#[cfg(test)]
fn spawn_counting_server(
  map: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<phext::Coordinate, String>>>,
  log: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
) -> u16 {
  use std::io::Write;
  let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
  let port = listener.local_addr().unwrap().port();
  std::thread::spawn(move || {
    for stream in listener.incoming() {
      let mut stream = match stream { Ok(s) => s, Err(_) => continue };
      let map = map.clone();
      let log = log.clone();
      std::thread::spawn(move || {
        let request = match crate::read_http_request(&mut stream) { Ok(r) => r, Err(_) => return };
        let first_line = request.header.lines().next().unwrap_or("").to_string();
        log.lock().unwrap().push(first_line.clone());
        let reply = |stream: &mut std::net::TcpStream, status: u16, text: &str, body: &str| {
          let response = format!("HTTP/1.1 {} {}\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\n\r\n{}",
            status, text, body.len(), body);
          let _ = stream.write_all(response.as_bytes());
        };
        if first_line.starts_with("GET /api/v2/seed") {
          let body = crate::seed::build_seed(&map.lock().unwrap());
          reply(&mut stream, 200, "OK", &body);
        } else if first_line.starts_with("GET /api/v2/fetch") || first_line.starts_with("POST /api/v2/fetch") {
          let target = first_line.split_whitespace().nth(1).unwrap_or("");
          let query = target.split('?').nth(1).unwrap_or("");
          let parsed = crate::parse_query_string(query);
          let spec = if first_line.starts_with("POST") {
            let body = String::from_utf8_lossy(&request.content).to_string();
            crate::seed::parse_fetch_post(&body)
          } else {
            crate::seed::parse_fetch_get(&parsed)
          };
          match spec {
            Ok(spec) => {
              let body = crate::seed::run_fetch(&map.lock().unwrap(), &spec);
              reply(&mut stream, 200, "OK", &body);
            }
            Err(msg) => reply(&mut stream, 400, "Bad Request", &msg),
          }
        } else {
          reply(&mut stream, 404, "Not Found", "Not Found");
        }
      });
    }
  });
  port
}

#[test]
fn test_fetch_cached_second_fetch_transfers_no_content() {
  let map = std::sync::Arc::new(std::sync::Mutex::new(seed_fixture()));
  let log = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
  let port = spawn_counting_server(map.clone(), log.clone());
  let base = format!("http://127.0.0.1:{}", port);
  let coord = "1.1.1/1.1.1/1.1.1";
  let count = |log: &std::sync::Arc<std::sync::Mutex<Vec<String>>>, needle: &str|
    log.lock().unwrap().iter().filter(|l| l.contains(needle)).count();

  let cache_path = std::env::temp_dir().join(format!("sq-seed-cache-e2e-{}.json", std::process::id()));
  let cache_file = cache_path.to_string_lossy().to_string();
  let _ = std::fs::remove_file(&cache_file);

  // first fetch: the cache is empty, so the scroll crosses the wire
  let mut cache = crate::seed::ScrollCache::load(&cache_file);
  let (content, local) = crate::seed::fetch_cached(&base, "world", &None, coord, &mut cache).unwrap();
  assert_eq!(content, "Hello World!");
  assert_eq!(local, false);
  cache.save(&cache_file).unwrap();
  assert_eq!(count(&log, "/api/v2/seed"), 1);
  assert_eq!(count(&log, "/api/v2/fetch"), 1);

  // second fetch of the unchanged scroll: the seed is consulted again, but
  // the content endpoint is never touched — proven by the request log
  let mut cache = crate::seed::ScrollCache::load(&cache_file);
  let (content, local) = crate::seed::fetch_cached(&base, "world", &None, coord, &mut cache).unwrap();
  assert_eq!(content, "Hello World!");
  assert_eq!(local, true);
  assert_eq!(count(&log, "/api/v2/seed"), 2);
  assert_eq!(count(&log, "/api/v2/fetch"), 1, "second fetch must not touch the content endpoint");

  // the scroll changes server-side: the next fetch goes back to the wire,
  // verifies the new hash, and returns the new content
  map.lock().unwrap().insert(phext::to_coordinate(coord), "Hello World, revised!".to_string());
  let (content, local) = crate::seed::fetch_cached(&base, "world", &None, coord, &mut cache).unwrap();
  assert_eq!(content, "Hello World, revised!");
  assert_eq!(local, false);
  assert_eq!(count(&log, "/api/v2/fetch"), 2);
  let entry = cache.get(coord).unwrap();
  assert_eq!(entry.hash, crate::seed::scroll_hash("Hello World, revised!"));

  let _ = std::fs::remove_file(&cache_file);
}
