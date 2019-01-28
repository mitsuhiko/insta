use insta::{assert_debug_snapshot_matches, assert_ron_snapshot_matches};
use serde::Serialize;

#[test]
fn test_simple() {
    assert_debug_snapshot_matches!(vec![1, 2, 3], @r###"[
    1,
    2,
    3
]"###);
}

#[test]
fn test_complex() {
    assert_debug_snapshot_matches!(vec![1, 2, 3, 4, 5], @r###"[
    1,
    2,
    3,
    4,
    5
]"###);
}

#[test]
fn test_ron_inline() {
    #[derive(Serialize)]
    pub struct Email(String);

    #[derive(Serialize)]
    pub struct User {
        id: u32,
        username: String,
        email: Email,
    }

    assert_ron_snapshot_matches!(User {
        id: 42,
        username: "peter-doe".into(),
        email: Email("peter@doe.invalid".into()),
    }, @r###"User(
  id: 42,
  username: "peter-doe",
  email: Email("peter@doe.invalid"),
)"###);
}
