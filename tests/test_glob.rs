#![cfg(feature = "glob")]

#[test]
fn test_basic_globbing() {
    insta::glob!("inputs/*.txt", |path| {
        let contents = std::fs::read_to_string(path).unwrap();
        insta::assert_json_snapshot!(&contents);
    });
}

#[test]
fn test_that_should_probably_fail() {
    // FIXME: Ideally this would fail.
    let typod_glob = "inputz/*.txt";
    insta::glob!(typod_glob, |_| assert!(false));
}
