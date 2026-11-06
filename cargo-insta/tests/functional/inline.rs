use std::process::Stdio;

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

/// Regression test for https://github.com/mitsuhiko/insta/issues/865#issuecomment-2927572941
/// Existing snapshots with escaped format should not trigger unfixable legacy warnings.
#[test]
fn test_existing_escaped_snapshot_no_legacy_warning() {
    // This tests the exact case from the issue: an already-accepted snapshot
    // that previously triggered a "legacy format" warning that couldn't be fixed.
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "test_existing_escaped"
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
fn test_existing() {
    // This snapshot was already accepted - it should just pass, no warnings
    insta::assert_snapshot!("\n\r foo", @"\n\r foo");
}
"#
            .to_string(),
        )
        .create_project();

    let output = test_project.insta_cmd().args(["test"]).output().unwrap();

    assert!(
        output.status.success(),
        "Test should pass: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let combined = String::from_utf8_lossy(&output.stdout).to_string()
        + &String::from_utf8_lossy(&output.stderr);
    assert!(
        !combined.contains("legacy format"),
        "Should not show legacy format warning: {}",
        combined
    );
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

    // The snapshot should use escape sequences for the control characters.
    // Escaped format doesn't add a formatting newline (unlike block format).
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

    // Should NOT show legacy format warning (the unfixable warning bug)
    let combined = String::from_utf8_lossy(&output.stdout).to_string()
        + &String::from_utf8_lossy(&output.stderr);
    assert!(
        !combined.contains("legacy format"),
        "Should not show legacy format warning after fix: {}",
        combined
    );
}

/// With `review.inline_indentation: nested`, multiline values are written one
/// level deeper than the line the literal starts on.
#[test]
fn test_inline_indentation_nested() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_inline_indentation_nested")
        .add_file(
            "insta.yaml",
            r#"
review:
  inline_indentation: nested
"#
            .to_string(),
        )
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_new() {
    insta::assert_snapshot!("Tasks\n  write tape\n> run test", @"");
}

#[test]
fn test_multiline_call() {
    insta::assert_snapshot!(
        "a\nb",
        @""
    );
}

#[test]
fn test_hand_indented() {
    insta::assert_snapshot!("a\nnew", @"
        a
        old
    ");
}

#[test]
fn test_single_line() {
    insta::assert_snapshot!("a", @"");
}

#[test]
fn test_after_multiline_argument() {
    insta::assert_snapshot!(format!("{}", "a
b"), @"");
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
    @@ -1,14 +1,21 @@
     
     #[test]
     fn test_new() {
    -    insta::assert_snapshot!("Tasks\n  write tape\n> run test", @"");
    +    insta::assert_snapshot!("Tasks\n  write tape\n> run test", @"
    +        Tasks
    +          write tape
    +        > run test
    +    ");
     }
     
     #[test]
     fn test_multiline_call() {
         insta::assert_snapshot!(
             "a\nb",
    -        @""
    +        @"
    +            a
    +            b
    +        "
         );
     }
     
    @@ -16,17 +23,20 @@
     fn test_hand_indented() {
         insta::assert_snapshot!("a\nnew", @"
             a
    -        old
    +        new
         ");
     }
     
     #[test]
     fn test_single_line() {
    -    insta::assert_snapshot!("a", @"");
    +    insta::assert_snapshot!("a", @"a");
     }
     
     #[test]
     fn test_after_multiline_argument() {
         insta::assert_snapshot!(format!("{}", "a
    -b"), @"");
    +b"), @"
    +        a
    +        b
    +    ");
     }
    "#);

    // The written snapshots pass
    let output = test_project.insta_cmd().args(["test"]).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// `review.inline_indentation: nested` indents with a tab when the code does
#[test]
fn test_inline_indentation_nested_tabs() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_inline_indentation_nested_tabs")
        .add_file(
            "insta.yaml",
            r#"
review:
  inline_indentation: nested
"#
            .to_string(),
        )
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_tabs() {
    insta::assert_snapshot!("a\nb", @"");
}

#[test]
fn test_alignment_spaces_after_tab() {
    insta::assert_snapshot!(
    ..."a\nb",
    ...@""
    );
}
"#
            .replace("    ", "\t")
            .replace("...", "    "),
        )
        .create_project();

    let output = test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap();

    assert!(&output.status.success());

    assert_snapshot!(test_project.diff("src/lib.rs").replace('\t', "<tab>"), @r#"
    --- Original: src/lib.rs
    +++ Updated: src/lib.rs
    @@ -1,13 +1,19 @@
     
     #[test]
     fn test_tabs() {
    -<tab>insta::assert_snapshot!("a\nb", @"");
    +<tab>insta::assert_snapshot!("a\nb", @"
    +<tab><tab>a
    +<tab><tab>b
    +<tab>");
     }
     
     #[test]
     fn test_alignment_spaces_after_tab() {
     <tab>insta::assert_snapshot!(
     <tab>    "a\nb",
    -<tab>    @""
    +<tab>    @"
    +<tab>        a
    +<tab>        b
    +<tab>    "
     <tab>);
     }
    "#);
}

/// A `review.inline_indentation` that isn't a known name is an error, rather
/// than silently keeping the default
#[test]
fn test_inline_indentation_invalid() {
    for value in ["sideways", "true"] {
        let test_project = TestFiles::new()
            .add_cargo_toml("test_inline_indentation_invalid")
            .add_file(
                "insta.yaml",
                format!(
                    r#"
review:
  inline_indentation: {value}
"#
                ),
            )
            .add_file("src/lib.rs", "".to_string())
            .create_project();

        let output = test_project
            .insta_cmd()
            .args(["test"])
            .stdout(Stdio::piped())
            .output()
            .unwrap();

        assert!(!output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("invalid value for config 'inline_indentation'"),
            "Expected error mentioning inline_indentation for {value}, got: {stdout}"
        );
    }
}
