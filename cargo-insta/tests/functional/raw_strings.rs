use insta::assert_snapshot;

use crate::TestFiles;

/// Test case from https://github.com/mitsuhiko/insta/issues/827#issuecomment-3694405166
/// When a snapshot contains only newlines (no quotes or backslashes), the output
/// should use a regular string, not a raw string.
#[test]
fn test_multiline_no_special_chars_uses_regular_string() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_multiline_regular_string")
        .add_file(
            "src/lib.rs",
            r#####"
#[test]
fn test() {
    let result = "a\nb\n";
    insta::assert_snapshot!(result.to_string(), @"");
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

    // The snapshot should use a regular string (not raw) since the content
    // only contains newlines, no quotes or backslashes
    assert_snapshot!(test_project.diff("src/lib.rs"), @r#"
    --- Original: src/lib.rs
    +++ Updated: src/lib.rs
    @@ -2,5 +2,8 @@
     #[test]
     fn test() {
         let result = "a\nb\n";
    -    insta::assert_snapshot!(result.to_string(), @"");
    +    insta::assert_snapshot!(result.to_string(), @"
    +    a
    +    b
    +    ");
     }
    "#);
}

/// Test that needless raw strings work as inputs and are converted to regular strings in outputs
#[test]
fn test_needless_raw_strings_conversion() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_needless_raw_strings")
        .add_file(
            "src/lib.rs",
            r#####"
#[test]
fn test_single_line() {
    // These raw strings don't contain backslashes or quotes, so they're needless
    insta::assert_snapshot!(r#"single line should fit on a single line"#, @"");
    insta::assert_snapshot!(r##"single line should fit on a single line, even if it's really really really really really really really really really long"##, @"");
}

#[test]
fn test_multiline_only() {
    // Multiline content without quotes or backslashes
    insta::assert_snapshot!(r#"multiline content starting on first line

    final line
    "#, @"");
}

#[test]
fn test_with_quotes_needs_raw() {
    // This one needs raw strings because it contains quotes
    insta::assert_snapshot!(r#"content with "quotes""#, @"");
}

#[test]
fn test_with_backslash_needs_raw() {
    // This one needs raw strings because it contains backslashes
    insta::assert_snapshot!(r"content with \backslash", @"");
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

    // Verify that needless raw strings are converted to regular strings,
    // but necessary raw strings are preserved
    assert_snapshot!(test_project.diff("src/lib.rs"), @r###"
    --- Original: src/lib.rs
    +++ Updated: src/lib.rs
    @@ -2,8 +2,8 @@
     #[test]
     fn test_single_line() {
         // These raw strings don't contain backslashes or quotes, so they're needless
    -    insta::assert_snapshot!(r#"single line should fit on a single line"#, @"");
    -    insta::assert_snapshot!(r##"single line should fit on a single line, even if it's really really really really really really really really really long"##, @"");
    +    insta::assert_snapshot!(r#"single line should fit on a single line"#, @"single line should fit on a single line");
    +    insta::assert_snapshot!(r##"single line should fit on a single line, even if it's really really really really really really really really really long"##, @"single line should fit on a single line, even if it's really really really really really really really really really long");
     }
     
     #[test]
    @@ -12,17 +12,21 @@
         insta::assert_snapshot!(r#"multiline content starting on first line
     
         final line
    -    "#, @"");
    +    "#, @"
    +    multiline content starting on first line
    +
    +        final line
    +    ");
     }
     
     #[test]
     fn test_with_quotes_needs_raw() {
         // This one needs raw strings because it contains quotes
    -    insta::assert_snapshot!(r#"content with "quotes""#, @"");
    +    insta::assert_snapshot!(r#"content with "quotes""#, @r#"content with "quotes""#);
     }
     
     #[test]
     fn test_with_backslash_needs_raw() {
         // This one needs raw strings because it contains backslashes
    -    insta::assert_snapshot!(r"content with \backslash", @"");
    +    insta::assert_snapshot!(r"content with \backslash", @r"content with \backslash");
     }
    "###);
}

/// Test YAML format with multiline content (no quotes or backslashes)
#[test]
fn test_yaml_multiline_needless_raw() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "test_yaml_needless_raw"
version = "0.1.0"
edition = "2021"

[dependencies]
insta = { path = '$PROJECT_PATH', features=["yaml"] }
"#
            .to_string(),
        )
        .add_file(
            "src/lib.rs",
            r#####"
#[test]
fn test_yaml_output() {
    // Input uses needless raw string, output should be regular string
    insta::assert_snapshot!(r#"---
This is invalid yaml:
 {
    {
---
    "#, @"");
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

    // The output should use regular strings (not raw) since it doesn't contain quotes or backslashes
    assert_snapshot!(test_project.diff("src/lib.rs"), @r##"
    --- Original: src/lib.rs
    +++ Updated: src/lib.rs
    @@ -7,5 +7,11 @@
      {
         {
     ---
    -    "#, @"");
    +    "#, @"
    +    ---
    +    This is invalid yaml:
    +     {
    +        {
    +    ---
    +    ");
     }
    "##);
}
