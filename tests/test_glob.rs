#![cfg(feature = "glob")]

mod glob_submodule;

#[test]
fn test_basic_globbing() {
    insta::glob!("inputs/*.txt", |path| {
        let contents = std::fs::read_to_string(path).unwrap();
        insta::assert_json_snapshot!(&contents);
    });
}

#[test]
fn test_basic_globbing_nested() {
    insta::glob!("inputs-nested/*/*.txt", |path| {
        let contents = std::fs::read_to_string(path).unwrap();
        insta::assert_snapshot!(&contents);
    });
}

#[test]
fn test_globs_follow_links() {
    insta::glob!("link-to-inputs/*.txt", |path| {
        let contents = std::fs::read_to_string(path).unwrap();
        insta::assert_json_snapshot!(&contents);
    });
}

#[test]
#[should_panic(expected = "the glob! macro did not match any files.")]
fn test_empty_glob_fails() {
    insta::glob!("nonexistent", |_| {
        // nothing
    });
}
