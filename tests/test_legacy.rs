#![allow(deprecated)]
use insta::{
    assert_debug_snapshot_matches, assert_display_snapshot_matches, assert_json_snapshot_matches, assert_yaml_snapshot_matches,
};
use std::fmt;

#[test]
fn test_legacy_debug_vector() {
    assert_debug_snapshot_matches!("debug_vector", vec![1, 2, 3]);
}

#[test]
fn test_legacy_unnamed_debug_vector() {
    assert_debug_snapshot_matches!(vec![1, 2, 3]);
    assert_debug_snapshot_matches!(vec![1, 2, 3, 4]);
    assert_debug_snapshot_matches!(vec![1, 2, 3, 4, 5]);
}

#[test]
fn test_legacy_yaml_vector() {
    assert_yaml_snapshot_matches!("yaml_vector", vec![1, 2, 3]);
}

#[test]
fn test_legacy_unnamed_yaml_vector() {
    assert_yaml_snapshot_matches!(vec![1, 2, 3]);
    assert_yaml_snapshot_matches!(vec![1, 2, 3, 4]);
    assert_yaml_snapshot_matches!(vec![1, 2, 3, 4, 5]);
}

#[test]
fn test_legacy_json_vector() {
    assert_json_snapshot_matches!("json_vector", vec![1, 2, 3]);
}

#[test]
fn test_legacy_unnamed_json_vector() {
    assert_json_snapshot_matches!(vec![1, 2, 3]);
    assert_json_snapshot_matches!(vec![1, 2, 3, 4]);
    assert_json_snapshot_matches!(vec![1, 2, 3, 4, 5]);
}

struct TestDisplay;

impl fmt::Display for TestDisplay {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "TestDisplay struct")
    }
}

#[test]
fn test_legacy_display() {
    let td = TestDisplay;
    assert_display_snapshot_matches!("display", td);
}

#[test]
fn test_legacy_unnamed_display() {
    let td = TestDisplay;
    assert_display_snapshot_matches!(td);
    assert_display_snapshot_matches!("whatever");
}
