use insta::assert_glob_snapshot;

#[test]
fn identity_glob_example() {
    assert_glob_snapshot!("test_data/*.txt", std::convert::identity);
}
