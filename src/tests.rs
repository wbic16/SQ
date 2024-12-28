#[cfg(test)]
use libphext::phext;

#[test]
fn test_insert() {
  let mut scroll = String::new();
  let command = "insert".to_string();
  let mut buffer = String::new();
  let coordinate = phext::to_coordinate("1.1.1/1.1.1/1.1.2");
  let update = "Hello World!".to_string();
  let filename = "insert.phext".to_string();
  let done = crate::sq::process(&mut scroll, command, &mut buffer, coordinate, update, filename);

  assert_eq!(buffer, "\x17Hello World!");
  assert_eq!(done, false);
}

#[test]
fn test_select() {
  let mut scroll = String::new();
  let command = "select".to_string();
  let mut buffer = "\x17\x17Third Scroll Content".to_string();
  let coordinate = phext::to_coordinate("1.1.1/1.1.1/1.1.3");
  let update = "ignored text".to_string();
  let filename = "select.phext".to_string();
  let done = crate::sq::process(&mut scroll, command, &mut buffer, coordinate, update, filename);

  assert_eq!(buffer, "\x17\x17Third Scroll Content");
  assert_eq!(scroll, "Third Scroll Content");
  assert_eq!(done, false);
}

#[test]
fn test_update() {
  let mut scroll = String::new();
  let command = "update".to_string();
  let mut buffer = "\x17\x18\x17Third Scroll Original".to_string();
  let coordinate = phext::to_coordinate("1.1.1/1.1.1/1.2.2");
  let update = "Full Rewrite at 1.2.2".to_string();
  let filename = "update.phext".to_string();
  let done = crate::sq::process(&mut scroll, command, &mut buffer, coordinate, update, filename);

  assert_eq!(buffer, "\x17\x18\x17Full Rewrite at 1.2.2");
  assert_eq!(scroll, "Updated 21 bytes");
  assert_eq!(done, false);
}

#[test]
fn test_delete() {
  let mut scroll = String::new();
  let command = "delete".to_string();
  let mut buffer = "\x17\x18\x17Third Scroll Original".to_string();
  let coordinate = phext::to_coordinate("1.1.1/1.1.1/1.2.2");
  let update = "".to_string();
  let filename = "delete.phext".to_string();
  let done = crate::sq::process(&mut scroll, command, &mut buffer, coordinate, update, filename);

  assert_eq!(buffer, "\x17\x18\x17");
  assert_eq!(scroll, "Removed 21 bytes");
  assert_eq!(done, false);
}

#[test]
fn test_save() {
  let mut scroll = String::new();
  let command = "save".to_string();
  let mut buffer = "\x17\x18\x17Save Test".to_string();
  let coordinate = phext::to_coordinate("1.1.1/1.1.1/1.2.2");
  let update = "Save Test at 1.2.2".to_string();
  let filename = "save.phext".to_string();
  let done = crate::sq::process(&mut scroll, command, &mut buffer, coordinate, update, filename);

  assert_eq!(buffer, "\x17\x18\x17Save Test");
  assert_eq!(scroll, "Wrote 12 bytes to save.phext");
  assert_eq!(done, false);

  std::fs::remove_file("save.phext").expect("Unable to find save.phext");
}

#[test]
fn test_exit() {
  let mut scroll = String::new();
  let command = "shutdown".to_string();
  let mut buffer = "".to_string();
  let coordinate = phext::to_coordinate("1.1.1/1.1.1/1.1.1");
  let update = "Shutdown Test".to_string();
  let filename = "shutdown.phext".to_string();
  let done = crate::sq::process(&mut scroll, command, &mut buffer, coordinate, update, filename);

  assert_eq!(done, true);
}