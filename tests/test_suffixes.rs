#[cfg(feature = "json")]
#[test]
fn test_basic_suffixes() {
    for value in [1, 2, 3] {
        insta::with_settings!({snapshot_suffix => value.to_string()}, {
            insta::assert_json_snapshot!(&value);
        });
    }
}
