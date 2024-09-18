#[test]
fn test_binary_snapshot() {
    insta::assert_binary_snapshot!("txt", b"test".to_vec());
}
