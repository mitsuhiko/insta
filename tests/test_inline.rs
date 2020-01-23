#[cfg(feature = "ron")]
use insta::assert_ron_snapshot;
use insta::{assert_debug_snapshot, assert_json_snapshot, assert_snapshot, assert_yaml_snapshot};
use serde::Serialize;
use std::thread;

#[test]
fn test_simple() {
    assert_debug_snapshot!(vec![1, 2, 3, 4], @r###"
    [
        1,
        2,
        3,
        4,
    ]
    "###);
}

#[test]
fn test_single_line() {
    assert_snapshot!("Testing", @"Testing");
}

#[test]
fn test_unnamed_single_line() {
    assert_snapshot!("Testing");
    assert_snapshot!("Testing-2");
}

#[test]
fn test_unnamed_thread_single_line() {
    let builder = thread::Builder::new().name("foo::lol::something".into());

    let handler = builder
        .spawn(|| {
            assert_snapshot!("Testing-thread");
            assert_snapshot!("Testing-thread-2");
        })
        .unwrap();

    handler.join().unwrap();
}

#[test]
fn test_newline() {
    // https://github.com/mitsuhiko/insta/issues/39
    assert_snapshot!("\n", @"
");
}

#[cfg(feature = "ron")]
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

    assert_ron_snapshot!(User {
        id: 42,
        username: "peter-doe".into(),
        email: Email("peter@doe.invalid".into()),
    }, @r###"
    User(
      id: 42,
      username: "peter-doe",
      email: Email("peter@doe.invalid"),
    )
    "###);
}

#[test]
fn test_json_inline() {
    assert_json_snapshot!(vec!["foo", "bar"], @r###"
    [
      "foo",
      "bar"
    ]
    "###);
}

#[test]
fn test_yaml_inline() {
    #[derive(Serialize)]
    pub struct User {
        id: u32,
        username: String,
        email: String,
    }

    assert_yaml_snapshot!(User {
        id: 42,
        username: "peter-pan".into(),
        email: "peterpan@wonderland.invalid".into()
    }, @r###"
    ---
    id: 42
    username: peter-pan
    email: peterpan@wonderland.invalid
    "###);
}

#[cfg(feature = "redactions")]
#[test]
fn test_yaml_inline_redacted() {
    #[derive(Serialize)]
    pub struct User {
        id: u32,
        username: String,
        email: String,
    }

    assert_yaml_snapshot!(User {
        id: 42,
        username: "peter-pan".into(),
        email: "peterpan@wonderland.invalid".into()
    }, {
        ".id" => "[user-id]"
    }, @r###"
    ---
    id: "[user-id]"
    username: peter-pan
    email: peterpan@wonderland.invalid
    "###);
}

#[test]
fn test_non_basic_plane() {
    assert_snapshot!("a 😀oeu", @"a 😀oeu");
}
