use std::io::Write;

#[test]
fn test_binary_snapshot() {
    insta::assert_binary_snapshot!("txt", |file| {
        file.write_all(b"test").unwrap();
    });
}

#[test]
fn test_file_extension_collision() {
    // ubuntu snap packages also have the .snap extension so let's make sure that doesn't cause
    // problems
    insta::assert_binary_snapshot!("snap", |file| {
        file.write_all(b"test").unwrap();
    });
}

#[test]
fn test_file_empty_extension() {
    insta::assert_binary_snapshot!("", |file| {
        file.write_all(b"test").unwrap();
    });
}

#[test]
#[should_panic(expected = "this file extension is not allowed")]
fn test_new_extension() {
    insta::assert_binary_snapshot!("new", |_| {});
}

#[test]
#[should_panic(expected = "this file extension is not allowed")]
fn test_underscore_extension() {
    insta::assert_binary_snapshot!("_", |_| {});
}

#[test]
#[should_panic(expected = "file extensions starting with 'new.' are not allowed")]
fn test_extension_starting_with_new() {
    insta::assert_binary_snapshot!("new.gz", |_| {});
}

#[test]
fn test_multipart_extension() {
    insta::assert_binary_snapshot!("tar.gz", |file| {
        file.write_all(b"test").unwrap();
    });
}
