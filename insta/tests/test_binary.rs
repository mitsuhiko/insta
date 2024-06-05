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
