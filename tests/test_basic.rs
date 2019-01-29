extern crate insta;

use insta::{
    assert_debug_snapshot_matches, assert_json_snapshot_matches,
    assert_serialized_snapshot_matches, assert_yaml_snapshot_matches,
};

#[test]
fn test_vector() {
    assert_debug_snapshot_matches!("vector", vec![1, 2, 3]);
}

#[test]
fn test_serialized_vector() {
    assert_serialized_snapshot_matches!("serialized_vector", vec![1, 2, 3]);
}

#[test]
fn test_yaml_vector() {
    assert_yaml_snapshot_matches!("yaml_vector", vec![1, 2, 3]);
}

#[test]
fn test_json_vector() {
    assert_json_snapshot_matches!("json_json", vec![1, 2, 3]);
}
