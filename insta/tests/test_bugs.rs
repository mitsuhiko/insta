#[test]
fn test_crlf() {
    insta::assert_snapshot!("foo\r\nbar\r\nbaz");
}

#[test]
fn test_trailing_crlf() {
    insta::assert_snapshot!("foo\r\nbar\r\nbaz\r\n");
}

#[test]
fn test_trailing_crlf_inline() {
    insta::assert_snapshot!("foo\r\nbar\r\nbaz\r\n", @r###"
    foo
    bar
    baz
    "###);
}
