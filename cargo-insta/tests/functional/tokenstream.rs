//! Functional tests for TokenStream snapshot support.
//!
//! Tests the `assert_token_snapshot!` macro with both file-based and inline modes.

use insta::assert_snapshot;

use crate::TestFiles;

/// Test that inline TokenStream snapshots with empty `@{}` get updated correctly.
/// Note: When cargo-insta updates inline snapshots, it converts to string format `@"..."`
/// since that's the standard inline snapshot format.
#[test]
fn test_tokenstream_inline_empty_to_populated() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "test_tokenstream_inline_empty"
version = "0.1.0"
edition = "2021"

[dependencies]
insta = { path = '$PROJECT_PATH', features = ["tokenstream"] }
quote = "1.0"
proc-macro2 = "1.0"
"#
            .to_string(),
        )
        .add_file(
            "src/lib.rs",
            r#"
use proc_macro2::TokenStream;
use quote::quote;

#[test]
fn test_token_inline() {
    let tokens: TokenStream = quote! { struct Foo; };
    insta::assert_token_snapshot!(tokens, @{});
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

    assert!(
        output.status.success(),
        "Test failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify the @{} was updated - cargo-insta preserves brace format
    assert_snapshot!(test_project.diff("src/lib.rs"), @r#"
    --- Original: src/lib.rs
    +++ Updated: src/lib.rs
    @@ -5,5 +5,5 @@
     #[test]
     fn test_token_inline() {
         let tokens: TokenStream = quote! { struct Foo; };
    -    insta::assert_token_snapshot!(tokens, @{});
    +    insta::assert_token_snapshot!(tokens, @{ struct Foo; });
     }
    "#);
}

/// Test that inline TokenStream snapshots pass when tokens match semantically.
#[test]
fn test_tokenstream_inline_matching() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "test_tokenstream_inline_matching"
version = "0.1.0"
edition = "2021"

[dependencies]
insta = { path = '$PROJECT_PATH', features = ["tokenstream"] }
quote = "1.0"
proc-macro2 = "1.0"
"#
            .to_string(),
        )
        .add_file(
            "src/lib.rs",
            r#"
use proc_macro2::TokenStream;
use quote::quote;

#[test]
fn test_token_inline() {
    let tokens: TokenStream = quote! { struct Foo; };
    insta::assert_token_snapshot!(tokens, @{ struct Foo; });
}
"#
            .to_string(),
        )
        .create_project();

    let output = test_project
        .insta_cmd()
        .args(["test", "--", "--nocapture"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "Test should pass when tokens match: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // No changes should be made since tokens match
    assert_snapshot!(test_project.diff("src/lib.rs"), @"");
}

/// Test file-based TokenStream snapshots.
#[test]
fn test_tokenstream_file_based() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "test_tokenstream_file_based"
version = "0.1.0"
edition = "2021"

[dependencies]
insta = { path = '$PROJECT_PATH', features = ["tokenstream"] }
quote = "1.0"
proc-macro2 = "1.0"
"#
            .to_string(),
        )
        .add_file(
            "src/lib.rs",
            r#"
use proc_macro2::TokenStream;
use quote::quote;

#[test]
fn test_token_file() {
    let tokens: TokenStream = quote! {
        fn hello() {
            println!("Hello, world!");
        }
    };
    insta::assert_token_snapshot!("my_function", tokens);
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

    assert!(
        output.status.success(),
        "Test failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify the snapshot file was created
    assert_snapshot!(test_project.file_tree_diff(), @r#"
    --- Original file tree
    +++ Updated file tree
    @@ -1,3 +1,6 @@
    +  Cargo.lock
       Cargo.toml
       src
         src/lib.rs
    +    src/snapshots
    +      src/snapshots/test_tokenstream_file_based__my_function.snap
    "#);
}

/// Test that TokenStream comparison is semantic (ignores whitespace differences).
#[test]
fn test_tokenstream_semantic_comparison() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "test_tokenstream_semantic"
version = "0.1.0"
edition = "2021"

[dependencies]
insta = { path = '$PROJECT_PATH', features = ["tokenstream"] }
quote = "1.0"
proc-macro2 = "1.0"
"#
            .to_string(),
        )
        .add_file(
            "src/lib.rs",
            r#"
use proc_macro2::TokenStream;
use quote::quote;

#[test]
fn test_semantic() {
    // Extra whitespace in the quote! should still match
    let tokens: TokenStream = quote! {
        struct    Foo   {
            x  :  i32
        }
    };
    // Reference has different whitespace but same structure
    insta::assert_token_snapshot!(tokens, @{ struct Foo { x: i32 } });
}
"#
            .to_string(),
        )
        .create_project();

    let output = test_project
        .insta_cmd()
        .args(["test", "--", "--nocapture"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "Semantic comparison should pass despite whitespace differences: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // No changes should be made since tokens are semantically equal
    assert_snapshot!(test_project.diff("src/lib.rs"), @"");
}

/// Test multiline TokenStream inline snapshot.
#[test]
fn test_tokenstream_multiline_inline() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "test_tokenstream_multiline"
version = "0.1.0"
edition = "2021"

[dependencies]
insta = { path = '$PROJECT_PATH', features = ["tokenstream"] }
quote = "1.0"
proc-macro2 = "1.0"
"#
            .to_string(),
        )
        .add_file(
            "src/lib.rs",
            r#"
use proc_macro2::TokenStream;
use quote::quote;

#[test]
fn test_multiline() {
    let tokens: TokenStream = quote! {
        impl MyTrait for MyStruct {
            fn method(&self) -> i32 {
                42
            }
        }
    };
    insta::assert_token_snapshot!(tokens, @{});
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

    assert!(
        output.status.success(),
        "Test failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify the multiline tokens were inserted
    let diff = test_project.diff("src/lib.rs");
    assert!(
        diff.contains("impl MyTrait for MyStruct"),
        "Diff should contain the impl block"
    );
}
