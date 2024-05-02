use std::io::Write;

#[test]
fn test_binary_snapshot() {
    insta::assert_binary_snapshot!("bin", &mut |file| {
        file.write_all(&[0, 1, 2, 3]).unwrap();
    });
}
