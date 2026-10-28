use insta::assert_snapshot;
use std::process::Stdio;

use crate::TestFiles;

fn cargo_toml(name: &str, edition: &str) -> String {
    format!(
        r#"
[package]
name = "{name}"
version = "0.1.0"
edition = "{edition}"

[dependencies]
insta = {{ path = '$PROJECT_PATH' }}
"#
    )
}

/// Since edition 2024 rustdoc merges doctests into one crate; insta must
/// still recognize them as doctests.
#[test]
fn test_doctest_edition_2024_inline_warns() {
    let test_project = TestFiles::new()
        .add_file("Cargo.toml", cargo_toml("doctest_2024_inline", "2024"))
        .add_file(
            "src/lib.rs",
            r#"
//! ```
//! insta::assert_snapshot!("new", @"old");
//! ```
"#
            .to_string(),
        )
        .create_project();

    let output = test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("warning: cannot update inline snapshots in doctests"),
        "Expected warning not found in stderr:\n{stderr}"
    );
    assert_snapshot!(test_project.file_tree_diff(), @"
    --- Original file tree
    +++ Updated file tree
    @@ -1,3 +1,4 @@
    +  Cargo.lock
       Cargo.toml
       src
         src/lib.rs
    ");
}

/// A merged doctest's `file!()` is a temporary file, so a file snapshot would
/// be stored there and lost.
#[test]
fn test_doctest_edition_2024_file_snapshot_fails() {
    let test_project = TestFiles::new()
        .add_file("Cargo.toml", cargo_toml("doctest_2024_file", "2024"))
        .add_file(
            "src/lib.rs",
            r#"
//! ```
//! insta::assert_snapshot!("named", "value");
//! ```
"#
            .to_string(),
        )
        .create_project();

    let output = test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Cannot locate snapshot files in merged doctests"),
        "Expected panic not found in stdout:\n{stdout}"
    );
}

/// `standalone_crate` doctests are compiled as before edition 2024, so file
/// snapshots work.
#[test]
fn test_doctest_edition_2024_standalone_file_snapshot() {
    let test_project = TestFiles::new()
        .add_file("Cargo.toml", cargo_toml("doctest_2024_standalone", "2024"))
        .add_file(
            "src/lib.rs",
            r#"
//! ```standalone_crate
//! insta::assert_snapshot!("named", "value");
//! ```
"#
            .to_string(),
        )
        .create_project();

    let output = test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_snapshot!(test_project.file_tree_diff(), @"
    --- Original file tree
    +++ Updated file tree
    @@ -1,3 +1,6 @@
    +  Cargo.lock
       Cargo.toml
       src
         src/lib.rs
    +    src/snapshots
    +      src/snapshots/doctest_lib_rs__named.snap
    ");
}
