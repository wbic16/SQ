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
  let done = crate::sq::process(1, "memory".to_string(), &mut scroll, command, &mut map, coordinate, update, filename);
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
  let done = crate::sq::process(1, "memory".to_string(), &mut scroll, command, &mut map, coordinate, update, filename);

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
  let done = crate::sq::process(1, "memory".to_string(), &mut scroll, command, &mut map, coordinate, update, filename);
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
  let done = crate::sq::process(1, "memory".to_string(), &mut scroll, command, &mut map, coordinate, update, filename);
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
  let done = crate::sq::process(1, "memory".to_string(), &mut scroll, command, &mut map, coordinate, update, filename);
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
fn convert_from_csv() {
  let csv = "Field 1,Field 2,Field 3\nalpha,beta,gamma\n1,2,3\na,b,c";
  //let phext = phext::csv_convert(csv);
  
  // dates -> 3 coordinates
  // numbers -> compressed coordinates
  // lookup tables otherwise

  // example
  // 2.1.1/1.1.1/1.1.1 Field 1
  // 2.1.1/1.1.1/1.2.1 Field 2
  // 2.1.1/1.1.1/1.3.1 Field 3
  // 3.1.1/1.1.1/1.1.1 alpha
  // 3.1.1/1.1.1/1.1.2 beta
  // 3.1.1/1.1.1/1.1.3 gamma
  // 3.1.1/1.1.1/1.2.1 1
  // 3.1.1/1.1.1/1.2.2 2
  // 3.1.1/1.1.1/1.2.3 3
  // 3.1.1/1.1.1/1.3.1 a
  // 3.1.1/1.1.1/1.3.2 b
  // 3.1.1/1.1.1/1.3.3 c
  
  // the initial pass just maps inputs to a coordinate
}

#[test]
fn test_exit() {
  let mut scroll = String::new();
  let command = "shutdown".to_string();
  let mut buffer = phext::explode("");
  let coordinate = phext::to_coordinate("1.1.1/1.1.1/1.1.1");
  let update = "Shutdown Test".to_string();
  let filename = "shutdown.phext".to_string();

  let done = crate::sq::process(1, "memory".to_string(), &mut scroll, command, &mut buffer, coordinate, update, filename);

  assert_eq!(done, true);
}