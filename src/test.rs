#[test]
fn test_embedded_test() {
    assert_snapshot_matches!("embedded", "Just a string");
}