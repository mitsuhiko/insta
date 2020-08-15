#![cfg(feature = "glob")]

#[test]
fn test_basic_globbing() {
    insta::glob!("inputs/*.txt", |path| {
        let contents = std::fs::read_to_string(path).unwrap();
        insta::assert_json_snapshot!(&contents);
    });
}
