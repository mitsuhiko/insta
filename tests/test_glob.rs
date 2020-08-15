#![cfg(feature = "glob")]

#[test]
fn test_basic_globbing() {
    insta::glob!("inputs/*.txt", |path| {
        let contents = std::fs::read_to_string(path).unwrap();
        insta::assert_json_snapshot!(&contents);
    });
}

#[test]
#[should_panic(expected = "the glob \"inputz/*.txt\" did not match anything")]
fn test_glob_must_have_matches() {
    let typod_glob = "inputz/*.txt";
    insta::glob!(typod_glob, |_| assert!(false));
}
