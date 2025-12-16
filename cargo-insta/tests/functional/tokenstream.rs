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

/// Test multiline TokenStream inline snapshot with proper indentation.
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

    // Verify the multiline tokens were inserted with proper indentation
    assert_snapshot!(test_project.diff("src/lib.rs"), @r"
    --- Original: src/lib.rs
    +++ Updated: src/lib.rs
    @@ -11,5 +11,11 @@
                 }
             }
         };
    -    insta::assert_token_snapshot!(tokens, @{});
    +    insta::assert_token_snapshot!(tokens, @{
    +        impl MyTrait for MyStruct {
    +            fn method(&self) -> i32 {
    +                42
    +            }
    +        }
    +    });
     }
    ");
}

/// Test that single-line TokenStream uses compact format @{ content }.
#[test]
fn test_tokenstream_single_line_compact_format() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "test_tokenstream_single_line"
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
fn test_single() {
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

    // Verify single-line content stays compact on one line
    assert_snapshot!(test_project.diff("src/lib.rs"), @r#"
    --- Original: src/lib.rs
    +++ Updated: src/lib.rs
    @@ -5,5 +5,5 @@
     #[test]
     fn test_single() {
         let tokens: TokenStream = quote! { struct Foo; };
    -    insta::assert_token_snapshot!(tokens, @{});
    +    insta::assert_token_snapshot!(tokens, @{ struct Foo; });
     }
    "#);
}

/// Test that multiline TokenStream content gets proper indentation formatting.
#[test]
fn test_tokenstream_multiline_indentation() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "test_tokenstream_multiline_indent"
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
fn test_multiline_indent() {
    let tokens: TokenStream = quote! {
        pub struct Foo;
        pub struct Bar;
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

    // Verify multiline content is properly indented:
    // - Opening brace { on its own line after @
    // - Each content line indented 4 spaces beyond @{ line's leading whitespace
    // - Closing brace } aligns with @{ line's leading whitespace
    assert_snapshot!(test_project.diff("src/lib.rs"), @r"
    --- Original: src/lib.rs
    +++ Updated: src/lib.rs
    @@ -8,5 +8,8 @@
             pub struct Foo;
             pub struct Bar;
         };
    -    insta::assert_token_snapshot!(tokens, @{});
    +    insta::assert_token_snapshot!(tokens, @{
    +        pub struct Foo;
    +        pub struct Bar;
    +    });
     }
    ");
}

/// Test that multiline TokenStream with @{ on separate line gets deeper indentation.
#[test]
fn test_tokenstream_multiline_separate_line() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "test_tokenstream_separate_line"
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
fn test_separate() {
    let tokens: TokenStream = quote! {
        pub struct Foo;
        pub struct Bar;
    };
    // Multi-line macro call with @{ on a separate, more indented line
    insta::assert_token_snapshot!(
        tokens,
        @{}
    );
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

    // When @{ is on a separate line (indented 8 spaces), content gets 12 spaces
    // and closing brace gets 8 spaces (aligns with @{)
    assert_snapshot!(test_project.diff("src/lib.rs"), @r"
    --- Original: src/lib.rs
    +++ Updated: src/lib.rs
    @@ -11,6 +11,9 @@
         // Multi-line macro call with @{ on a separate, more indented line
         insta::assert_token_snapshot!(
             tokens,
    -        @{}
    +        @{
    +            pub struct Foo;
    +            pub struct Bar;
    +        }
         );
     }
    ");
}

/// Test TokenStream snapshot with non-expression tokens (fallback to TokenStream::to_string).
/// Tokens like `Vec<u8>` are not valid expressions or items, so pretty-printing falls back.
#[test]
fn test_tokenstream_non_expression_fallback() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "test_tokenstream_non_expr"
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
fn test_non_expr() {
    // Vec<u8> is not a valid expression or item, falls back to TokenStream::to_string()
    let tokens: TokenStream = quote! { Vec<u8> };
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

    // TokenStream::to_string() adds spaces around angle brackets
    assert_snapshot!(test_project.diff("src/lib.rs"), @r#"
    --- Original: src/lib.rs
    +++ Updated: src/lib.rs
    @@ -6,5 +6,5 @@
     fn test_non_expr() {
         // Vec<u8> is not a valid expression or item, falls back to TokenStream::to_string()
         let tokens: TokenStream = quote! { Vec<u8> };
    -    insta::assert_token_snapshot!(tokens, @{});
    +    insta::assert_token_snapshot!(tokens, @{ Vec < u8 > });
     }
    "#);
}

/// Test that non-expression tokens compare semantically (whitespace ignored).
#[test]
fn test_tokenstream_non_expression_semantic_comparison() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "test_tokenstream_non_expr_semantic"
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
fn test_non_expr_semantic() {
    // The snapshot has spaces but quote! normalizes them - should still match
    let tokens: TokenStream = quote! { Vec<u8> };
    insta::assert_token_snapshot!(tokens, @{ Vec < u8 > });
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
        "Non-expression tokens should match semantically: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // No changes - tokens match semantically
    assert_snapshot!(test_project.diff("src/lib.rs"), @"");
}

/// Test that untokenizable content inside @{} causes a compilation error.
/// An unclosed string literal cannot be tokenized.
#[test]
fn test_tokenstream_unclosed_string_fails_to_compile() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "test_tokenstream_invalid"
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
            r####"
use proc_macro2::TokenStream;
use quote::quote;

#[test]
fn test_invalid() {
    let tokens: TokenStream = quote! { struct Foo; };
    // Unclosed string literal - should fail to compile
    insta::assert_token_snapshot!(tokens, @{ "unclosed string });
}
"####
                .to_string(),
        )
        .create_project();

    // Run cargo build -q to capture only compilation errors
    let output = std::process::Command::new("cargo")
        .args(["build", "-q"])
        .current_dir(&test_project.workspace_dir)
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_snapshot!(stderr, @r#"
    error[E0765]: unterminated double quote string
      --> src/lib.rs:9:46
       |
     9 |       insta::assert_token_snapshot!(tokens, @{ "unclosed string });
       |  ______________________________________________^
    10 | | }
       | |__^

    For more information about this error, try `rustc --explain E0765`.
    error: could not compile `test_tokenstream_invalid` (lib) due to 1 previous error
    "#);
}

/// Test that unclosed delimiters inside @{} cause a compilation error.
#[test]
fn test_tokenstream_unclosed_delimiter_fails_to_compile() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "test_tokenstream_unclosed"
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
fn test_unclosed() {
    let tokens: TokenStream = quote! { struct Foo; };
    // Unclosed paren should fail to compile
    insta::assert_token_snapshot!(tokens, @{ fn foo( });
}
"#
            .to_string(),
        )
        .create_project();

    // Run cargo build -q to capture only compilation errors
    let output = std::process::Command::new("cargo")
        .args(["build", "-q"])
        .current_dir(&test_project.workspace_dir)
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_snapshot!(stderr, @r"
    error: mismatched closing delimiter: `}`
     --> src/lib.rs:9:52
      |
    9 |     insta::assert_token_snapshot!(tokens, @{ fn foo( });
      |                                            -       ^ ^ mismatched closing delimiter
      |                                            |       |
      |                                            |       unclosed delimiter
      |                                            closing delimiter possibly meant for this

    error: could not compile `test_tokenstream_unclosed` (lib) due to 1 previous error
    ");
}
