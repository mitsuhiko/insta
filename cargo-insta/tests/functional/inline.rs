use insta::assert_snapshot;

use crate::TestFiles;

#[test]
fn test_json_inline() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "test_json_inline"
version = "0.1.0"
edition = "2021"

[dependencies]
insta = { path = '$PROJECT_PATH', features=["json", "redactions"] }
serde = { version = "1.0", features = ["derive"] }
"#
            .to_string(),
        )
        .add_file(
            "src/lib.rs",
            r#"
use serde::Serialize;

#[allow(dead_code)]
#[derive(Serialize)]
struct User {
    id: u64,
    email: String,
}

#[test]
fn test_json_snapshot() {
    let user = User {
        id: 42,
        email: "john.doe@example.com".into(),
    };
    insta::assert_json_snapshot!(&user, {
        ".id" => "[user_id]",
    }, @"");
}
"#
            .to_string(),
        )
        .create_project();

    let output = test_project
        .insta_cmd()
        .args(["test", "--accept", "--", "--nocapture"])
        .output()
        .unwrap();

    assert!(&output.status.success());

    assert_snapshot!(test_project.diff("src/lib.rs"), @r##"
    --- Original: src/lib.rs
    +++ Updated: src/lib.rs
    @@ -16,5 +16,10 @@
         };
         insta::assert_json_snapshot!(&user, {
             ".id" => "[user_id]",
    -    }, @"");
    +    }, @r#"
    +    {
    +      "id": "[user_id]",
    +      "email": "john.doe@example.com"
    +    }
    +    "#);
     }
    "##);
}

#[test]
fn test_yaml_inline() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "test_yaml_inline"
version = "0.1.0"
edition = "2021"

[dependencies]
insta = { path = '$PROJECT_PATH', features=["yaml", "redactions"] }
serde = { version = "1.0", features = ["derive"] }
"#
            .to_string(),
        )
        .add_file(
            "src/lib.rs",
            r#"
use serde::Serialize;

#[allow(dead_code)]
#[derive(Serialize)]
struct User {
    id: u64,
    email: String,
}

#[test]
fn test_yaml_snapshot() {
    let user = User {
        id: 42,
        email: "john.doe@example.com".into(),
    };
    insta::assert_yaml_snapshot!(&user, {
        ".id" => "[user_id]",
    }, @"");
}
"#
            .to_string(),
        )
        .create_project();

    let output = test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap();

    assert!(&output.status.success());

    assert_snapshot!(test_project.diff("src/lib.rs"), @r##"
    --- Original: src/lib.rs
    +++ Updated: src/lib.rs
    @@ -16,5 +16,8 @@
         };
         insta::assert_yaml_snapshot!(&user, {
             ".id" => "[user_id]",
    -    }, @"");
    +    }, @r#"
    +    id: "[user_id]"
    +    email: john.doe@example.com
    +    "#);
     }
    "##);
}

#[test]
fn test_utf8_inline() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_utf8_inline")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_non_basic_plane() {
    /* an offset here ❄️ */ insta::assert_snapshot!("a 😀oeu", @"");
}

#[test]
fn test_remove_existing_value() {
    insta::assert_snapshot!("this is the new value", @"this is the old value");
}

#[test]
fn test_remove_existing_value_multiline() {
    insta::assert_snapshot!(
        "this is the new value",
        @"this is\
        this is the old value\
        it really is"
    );
}

#[test]
fn test_trailing_comma_in_inline_snapshot() {
    insta::assert_snapshot!(
        "new value",
        @"old value",  // comma here
    );
}
"#
            .to_string(),
        )
        .create_project();

    let output = test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap();

    assert!(&output.status.success());

    assert_snapshot!(test_project.diff("src/lib.rs"), @r#"
    --- Original: src/lib.rs
    +++ Updated: src/lib.rs
    @@ -1,21 +1,19 @@
     
     #[test]
     fn test_non_basic_plane() {
    -    /* an offset here ❄️ */ insta::assert_snapshot!("a 😀oeu", @"");
    +    /* an offset here ❄️ */ insta::assert_snapshot!("a 😀oeu", @"a 😀oeu");
     }
     
     #[test]
     fn test_remove_existing_value() {
    -    insta::assert_snapshot!("this is the new value", @"this is the old value");
    +    insta::assert_snapshot!("this is the new value", @"this is the new value");
     }
     
     #[test]
     fn test_remove_existing_value_multiline() {
         insta::assert_snapshot!(
             "this is the new value",
    -        @"this is\
    -        this is the old value\
    -        it really is"
    +        @"this is the new value"
         );
     }
     
    @@ -23,6 +21,6 @@
     fn test_trailing_comma_in_inline_snapshot() {
         insta::assert_snapshot!(
             "new value",
    -        @"old value",  // comma here
    +        @"new value",  // comma here
         );
     }
    "#);
}

/// Test the old format of inline YAML snapshots with a leading `---` still passes
#[test]
fn test_old_yaml_format() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "old-yaml-format"
version = "0.1.0"
edition = "2021"

[lib]
doctest = false

[dependencies]
insta = { path = '$PROJECT_PATH', features = ["yaml"] }
"#
            .to_string(),
        )
        .add_file(
            "src/lib.rs",
            r#####"
#[test]
fn test_old_yaml_format() {
    insta::assert_yaml_snapshot!("foo", @r####"
    ---
    foo
"####);
}
"#####
                .to_string(),
        )
        .create_project();

    // Check it passes
    assert!(test_project
        .insta_cmd()
        .args(["test", "--", "--nocapture"])
        .output()
        .unwrap()
        .status
        .success());
    // shouldn't be any changes
    assert_snapshot!(test_project.diff("src/lib.rs"), @"");

    // Also check that running with `--force-update-snapshots` updates the snapshot
    assert!(test_project
        .insta_cmd()
        .args(["test", "--force-update-snapshots", "--", "--nocapture"])
        .output()
        .unwrap()
        .status
        .success());

    assert_snapshot!(test_project.diff("src/lib.rs"), @r#####"
    --- Original: src/lib.rs
    +++ Updated: src/lib.rs
    @@ -1,8 +1,5 @@
     
     #[test]
     fn test_old_yaml_format() {
    -    insta::assert_yaml_snapshot!("foo", @r####"
    -    ---
    -    foo
    -"####);
    +    insta::assert_yaml_snapshot!("foo", @"foo");
     }
    "#####);
}

#[test]
fn test_hashtag_escape_in_inline_snapshot() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_hashtag_escape")
        .add_file(
            "src/lib.rs",
            r#####"
#[test]
fn test_hashtag_escape() {
    insta::assert_snapshot!(r###"Value with
    "## hashtags\n"###, @"");
}
"#####
                .to_string(),
        )
        .create_project();

    let output = test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap();

    assert!(&output.status.success());

    assert_snapshot!(test_project.diff("src/lib.rs"), @r####"
    --- Original: src/lib.rs
    +++ Updated: src/lib.rs
    @@ -2,5 +2,8 @@
     #[test]
     fn test_hashtag_escape() {
         insta::assert_snapshot!(r###"Value with
    -    "## hashtags\n"###, @"");
    +    "## hashtags\n"###, @r###"
    +    Value with
    +        "## hashtags\n
    +    "###);
     }
    "####);
}

#[test]
fn test_single_line_duplicates() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_single_line_duplicates")
        .add_file(
            "src/lib.rs",
            r#####"
#[test]
fn test_single_line_duplicates() {
    for _ in 0..2 {
        insta::allow_duplicates! {
            insta::assert_snapshot!("foo", @"");
        }
    }
}
"#####
                .to_string(),
        )
        .create_project();

    let output = test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap();

    assert!(&output.status.success());

    // diff shouldn't be applied twice
    assert_snapshot!(test_project.diff("src/lib.rs"), @r#"
    --- Original: src/lib.rs
    +++ Updated: src/lib.rs
    @@ -3,7 +3,7 @@
     fn test_single_line_duplicates() {
         for _ in 0..2 {
             insta::allow_duplicates! {
    -            insta::assert_snapshot!("foo", @"");
    +            insta::assert_snapshot!("foo", @"foo");
             }
         }
     }
    "#);
}

#[test]
fn test_single_line_assertions() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_single_line_assertions")
        .add_file(
            "src/lib.rs",
            r#####"
#[test]
fn test_single_line_assertions() {
    insta::assert_snapshot!("foo", @"");
    insta::assert_snapshot!("bar", @"");
}
"#####
                .to_string(),
        )
        .create_project();

    let output = test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap();

    assert!(&output.status.success());

    // all adjacent lines should be updated
    assert_snapshot!(test_project.diff("src/lib.rs"), @r#"
    --- Original: src/lib.rs
    +++ Updated: src/lib.rs
    @@ -1,6 +1,6 @@
     
     #[test]
     fn test_single_line_assertions() {
    -    insta::assert_snapshot!("foo", @"");
    -    insta::assert_snapshot!("bar", @"");
    +    insta::assert_snapshot!("foo", @"foo");
    +    insta::assert_snapshot!("bar", @"bar");
     }
    "#);
}

#[test]
fn test_multiple_assertions_within_allow_duplicates() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_multiple_assertions_within_allow_duplicates")
        .add_file(
            "src/lib.rs",
            r#####"
#[test]
fn test_multiple_assertions_within_allow_duplicates() {
    for _ in 0..2 {
        insta::allow_duplicates! {
            insta::assert_snapshot!("1", @"
            1a
            1b
            ");
            insta::assert_snapshot!("2", @"
            2a
            2b
            ");
        }
    }
}
"#####
                .to_string(),
        )
        .create_project();

    let output = test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap();

    assert!(&output.status.success());

    assert_snapshot!(test_project.diff("src/lib.rs"), @r#"
    --- Original: src/lib.rs
    +++ Updated: src/lib.rs
    @@ -3,14 +3,8 @@
     fn test_multiple_assertions_within_allow_duplicates() {
         for _ in 0..2 {
             insta::allow_duplicates! {
    -            insta::assert_snapshot!("1", @"
    -            1a
    -            1b
    -            ");
    -            insta::assert_snapshot!("2", @"
    -            2a
    -            2b
    -            ");
    +            insta::assert_snapshot!("1", @"1");
    +            insta::assert_snapshot!("2", @"2");
             }
         }
     }
    "#);
}

/// Regression test for https://github.com/mitsuhiko/insta/issues/865
/// Carriage return at the start of a line should be preserved, not treated as indentation.
#[test]
fn test_carriage_return_preserved() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "test_carriage_return"
version = "0.1.0"
edition = "2021"

[dependencies]
insta = { path = '$PROJECT_PATH' }
"#
            .to_string(),
        )
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_cr_preserved() {
    // Value with carriage return at start of second line
    insta::assert_snapshot!("\n\r foo", @"");
}
"#
            .to_string(),
        )
        .create_project();

    // Accept the snapshot
    let output = test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap();

    assert!(&output.status.success());

    // The snapshot should use escape sequences for the control characters
    assert_snapshot!(test_project.diff("src/lib.rs"), @r##"
    --- Original: src/lib.rs
    +++ Updated: src/lib.rs
    @@ -2,5 +2,5 @@
     #[test]
     fn test_cr_preserved() {
         // Value with carriage return at start of second line
    -    insta::assert_snapshot!("\n\r foo", @"");
    +    insta::assert_snapshot!("\n\r foo", @"\n\r foo");
     }
    "##);

    // Run again without --accept - should pass without needing to accept again
    // (This is the bug from issue #865: after accepting, the test would still fail)
    let output = test_project.insta_cmd().args(["test"]).output().unwrap();

    assert!(
        output.status.success(),
        "Test should pass after accepting snapshot (issue #865): {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
