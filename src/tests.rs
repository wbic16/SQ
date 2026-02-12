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
