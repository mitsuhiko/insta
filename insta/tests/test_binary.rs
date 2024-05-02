use std::io::Write;

#[test]
fn test_binary_snapshot() {
    insta::assert_binary_snapshot!("bin", &mut |file| {
        file.write_all(&[0, 1, 2, 3]).unwrap();
    });
}

#[test]
fn test_file_extension_collision() {
    // ubuntu snap packages also have the .snap extension so let's make sure that doesn't cause
    // problems
    insta::assert_binary_snapshot!("snap", &mut |file| {
        file.write_all(&[0, 1, 0, 1]).unwrap();
    });
}

#[test]
fn test_file_empty_extension() {
    insta::assert_binary_snapshot!("", &mut |file| {
        file.write_all(&[1, 3, 3, 7]).unwrap();
    });
}
