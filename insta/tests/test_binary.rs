#[test]
fn test_binary_snapshot() {
    insta::assert_binary_snapshot!("txt", b"test".to_vec());
}

#[test]
#[should_panic(expected = "this file extension is not allowed")]
fn test_new_extension() {
    insta::assert_binary_snapshot!("new", b"test".to_vec());
}

#[test]
#[should_panic(expected = "file extensions starting with 'new.' are not allowed")]
fn test_extension_starting_with_new() {
    insta::assert_binary_snapshot!("new.gz", b"test".to_vec());
}

#[test]
fn test_multipart_extension() {
    insta::assert_binary_snapshot!("tar.gz", b"test".to_vec());
}
