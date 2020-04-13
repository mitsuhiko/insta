#[test]
fn test_basic_suffixes() {
    for value in vec![1, 2, 3] {
        insta::with_settings!({snapshot_suffix => value.to_string()}, {
            insta::assert_json_snapshot!(&value);
        });
    }
}

#[cfg(feature = "glob")]
#[test]
fn test_basic_globbing() {
    insta::glob!("inputs/*.txt", |path| {
        let contents = std::fs::read_to_string(path).unwrap();
        insta::assert_json_snapshot!(&contents);
    });
}
