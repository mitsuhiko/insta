#[cfg(feature = "csv")]
use insta::assert_csv_snapshot;
#[cfg(feature = "ron")]
use insta::assert_ron_snapshot;
#[cfg(feature = "toml")]
use insta::assert_toml_snapshot;
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

// We used to use the thread name for snapshot name detection.  This is unreliable
// so this test now basically does exactly the same as `test_unnamed_single_line`.
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

#[cfg(feature = "csv")]
#[test]
fn test_csv_inline() {
    #[derive(Serialize)]
    pub struct Email(String);

    #[derive(Serialize)]
    pub struct User {
        id: u32,
        username: String,
        email: Email,
    }

    assert_csv_snapshot!(User {
        id: 1453,
        username: "mehmed-doe".into(),
        email: Email("mehmed@doe.invalid".into()),
    }, @r###"
    id,username,email
    1453,mehmed-doe,mehmed@doe.invalid
    "###);
}

#[cfg(feature = "csv")]
#[test]
fn test_csv_inline_multiple_values() {
    #[derive(Serialize)]
    pub struct Email(String);

    #[derive(Serialize)]
    pub struct User {
        id: u32,
        username: String,
        email: Email,
    }

    let user1 = User {
        id: 1453,
        username: "mehmed-doe".into(),
        email: Email("mehmed@doe.invalid".into()),
    };
    let user2 = User {
        id: 1455,
        username: "mehmed-doe-di".into(),
        email: Email("mehmed@doe-di.invalid".into()),
    };

    assert_csv_snapshot!(vec![user1, user2], @r###"
    id,username,email
    1453,mehmed-doe,mehmed@doe.invalid
    1455,mehmed-doe-di,mehmed@doe-di.invalid
    "###);
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

#[cfg(feature = "toml")]
#[test]
fn test_toml_inline() {
    #[derive(Serialize)]
    pub struct Email(String);

    #[derive(Serialize)]
    pub struct User {
        id: u32,
        username: String,
        email: Email,
    }

    assert_toml_snapshot!(User {
        id: 42,
        username: "peter-doe".into(),
        email: Email("peter@doe.invalid".into()),
    }, @r###"
    id = 42
    username = 'peter-doe'
    email = 'peter@doe.invalid'
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

#[test]
fn test_multiline_with_empty_lines() {
    assert_snapshot!("# first\nsecond\n  third\n\n# alternative", @r###"
    # first
    second
      third

    # alternative
    "###);
}
