extern crate insta;

use insta::{
    assert_debug_snapshot_matches,
    assert_display_snapshot_matches,
    assert_json_snapshot_matches,
    assert_yaml_snapshot_matches,
};
use std::fmt;

#[test]
fn test_debug_vector() {
    assert_debug_snapshot_matches!("debug_vector", vec![1, 2, 3]);
}

#[test]
fn test_yaml_vector() {
    assert_yaml_snapshot_matches!("yaml_vector", vec![1, 2, 3]);
}

#[test]
fn test_json_vector() {
    assert_json_snapshot_matches!("json_vector", vec![1, 2, 3]);
}

struct TestDisplay;

impl fmt::Display for TestDisplay {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "TestDisplay struct")
    }
}

#[test]
fn test_display() {
    let td = TestDisplay;
    assert_display_snapshot_matches!("display", td);
}