extern crate insta;

use insta::{assert_debug_snapshot_matches};

#[test]
fn test_vector() {
    assert_debug_snapshot_matches!("vector", vec![1, 2, 3]);
}

#[test]
#[cfg(feature = "serialization")]
fn test_serialized_vector() {
    use insta::assert_serialized_snapshot_matches;
    assert_serialized_snapshot_matches!("serialized_vector", vec![1, 2, 3]);
}
