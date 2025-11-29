//! Functional tests for the experimental `read_snapshot!` macro.

use std::fs;

use crate::TestFiles;

/// Tests reading a YAML snapshot that was created with `assert_yaml_snapshot!`
#[test]
fn test_read_yaml_snapshot() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "test_read_yaml"
version = "0.1.0"
edition = "2021"

[dependencies]
insta = { path = '$PROJECT_PATH', features = ["yaml"] }
"#
            .to_string(),
        )
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_read_yaml() {
    // Read the pre-existing YAML snapshot
    let content = insta::read_snapshot!("user_data").unwrap();

    // The content should be YAML-formatted
    assert!(content.contains("name:"), "Expected YAML format with 'name:' key");
    assert!(content.contains("Alice"), "Expected to find 'Alice' in snapshot");
    assert!(content.contains("age:"), "Expected YAML format with 'age:' key");
    assert!(content.contains("30"), "Expected to find '30' in snapshot");
}
"#
            .to_string(),
        )
        .add_file(
            "src/snapshots/test_read_yaml__user_data.snap",
            r#"---
source: src/lib.rs
expression: user
---
name: Alice
age: 30
"#
            .to_string(),
        )
        .create_project();

    // Run the test - it should pass since we're reading an existing snapshot
    let output = test_project
        .insta_cmd()
        .args(["test", "--", "--nocapture"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "Test failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Tests reading a JSON snapshot that was created with `assert_json_snapshot!`
#[test]
fn test_read_json_snapshot() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "test_read_json"
version = "0.1.0"
edition = "2021"

[dependencies]
insta = { path = '$PROJECT_PATH', features = ["json"] }
"#
            .to_string(),
        )
        .add_file(
            "src/lib.rs",
            r##"
#[test]
fn test_read_json() {
    // Read the pre-existing JSON snapshot
    let content = insta::read_snapshot!("api_response").unwrap();

    // The content should be JSON-formatted
    assert!(content.contains(r#""status":"#), "Expected JSON with 'status' key");
    assert!(content.contains(r#""ok""#), "Expected 'ok' value");
    assert!(content.contains(r#""data":"#), "Expected JSON with 'data' key");
}
"##
            .to_string(),
        )
        .add_file(
            "src/snapshots/test_read_json__api_response.snap",
            r#"---
source: src/lib.rs
expression: response
---
{
  "status": "ok",
  "data": [1, 2, 3]
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
        "Test failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Tests reading a plain text snapshot
#[test]
fn test_read_text_snapshot() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "test_read_text"
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
fn test_read_text() {
    let content = insta::read_snapshot!("greeting").unwrap();
    assert_eq!(content, "Hello, World!");
}
"#
            .to_string(),
        )
        .add_file(
            "src/snapshots/test_read_text__greeting.snap",
            r#"---
source: src/lib.rs
expression: msg
---
Hello, World!
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
        "Test failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Tests that reading a nonexistent snapshot returns an error
#[test]
fn test_read_nonexistent_snapshot() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "test_read_nonexistent"
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
fn test_nonexistent() {
    let result = insta::read_snapshot!("does_not_exist");
    assert!(result.is_err(), "Expected error for nonexistent snapshot");
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
        "Test failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Tests reading a snapshot with auto-generated name (based on function name)
#[test]
fn test_read_snapshot_auto_name() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "test_read_auto"
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
fn test_auto_named() {
    // Using read_snapshot!() without a name should use the function name
    let content = insta::read_snapshot!().unwrap();
    assert_eq!(content, "auto named content");
}
"#
            .to_string(),
        )
        .add_file(
            // Snapshot name derived from function name "test_auto_named" -> "auto_named"
            "src/snapshots/test_read_auto__auto_named.snap",
            r#"---
source: src/lib.rs
expression: value
---
auto named content
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
        "Test failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Tests reading a snapshot with custom snapshot_path setting
#[test]
fn test_read_snapshot_custom_path() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "test_read_custom_path"
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
fn test_custom_path() {
    insta::with_settings!({snapshot_path => "custom_snaps"}, {
        let content = insta::read_snapshot!("custom").unwrap();
        assert_eq!(content, "from custom path");
    });
}
"#
            .to_string(),
        )
        .add_file(
            "src/custom_snaps/test_read_custom_path__custom.snap",
            r#"---
source: src/lib.rs
expression: value
---
from custom path
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
        "Test failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Tests reading a binary snapshot
#[test]
fn test_read_binary_snapshot() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "test_read_binary"
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
fn test_binary() {
    let bytes = insta::read_binary_snapshot!("data.bin").unwrap();
    assert_eq!(bytes, vec![0x00, 0x01, 0x02, 0x03]);
}
"#
            .to_string(),
        )
        .add_file(
            "src/snapshots/test_read_binary__data.snap",
            r#"---
source: src/lib.rs
expression: bytes
snapshot_kind: binary
extension: bin
---
"#
            .to_string(),
        )
        .create_project();

    // Write the binary file separately (can't include binary in string)
    fs::write(
        test_project
            .workspace_dir
            .join("src/snapshots/test_read_binary__data.snap.bin"),
        [0x00, 0x01, 0x02, 0x03],
    )
    .unwrap();

    let output = test_project
        .insta_cmd()
        .args(["test", "--", "--nocapture"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "Test failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Tests reading a snapshot with snapshot_suffix setting
#[test]
fn test_read_snapshot_with_suffix() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "test_read_suffix"
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
fn test_suffix() {
    insta::with_settings!({snapshot_suffix => "linux"}, {
        let content = insta::read_snapshot!("platform_data").unwrap();
        assert_eq!(content, "linux-specific content");
    });
}
"#
            .to_string(),
        )
        .add_file(
            // Snapshot with @linux suffix
            "src/snapshots/test_read_suffix__platform_data@linux.snap",
            r#"---
source: src/lib.rs
expression: value
---
linux-specific content
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
        "Test failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Tests reading a snapshot with prepend_module_to_snapshot => false
#[test]
fn test_read_snapshot_no_module_prefix() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "test_read_no_prefix"
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
fn test_no_prefix() {
    insta::with_settings!({prepend_module_to_snapshot => false}, {
        let content = insta::read_snapshot!("no_prefix_data").unwrap();
        assert_eq!(content, "content without module prefix");
    });
}
"#
            .to_string(),
        )
        .add_file(
            // Snapshot WITHOUT module prefix (just name.snap)
            "src/snapshots/no_prefix_data.snap",
            r#"---
source: src/lib.rs
expression: value
---
content without module prefix
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
        "Test failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Tests reading a snapshot from a nested module
#[test]
fn test_read_snapshot_nested_module() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "test_read_nested"
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
mod parent {
    pub mod child {
        #[test]
        fn test_nested() {
            let content = insta::read_snapshot!("nested_data").unwrap();
            assert_eq!(content, "from nested module");
        }
    }
}
"#
            .to_string(),
        )
        .add_file(
            // Snapshot with nested module path: parent__child__nested_data.snap
            "src/snapshots/test_read_nested__parent__child__nested_data.snap",
            r#"---
source: src/lib.rs
expression: value
---
from nested module
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
        "Test failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Tests reading a snapshot with combined settings (suffix + custom path + no module prefix)
#[test]
fn test_read_snapshot_combined_settings() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "test_read_combined"
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
fn test_combined() {
    insta::with_settings!({
        snapshot_path => "custom",
        snapshot_suffix => "macos",
        prepend_module_to_snapshot => false
    }, {
        let content = insta::read_snapshot!("combined").unwrap();
        assert_eq!(content, "combined settings work");
    });
}
"#
            .to_string(),
        )
        .add_file(
            // Snapshot with all settings: custom/combined@macos.snap (no module prefix)
            "src/custom/combined@macos.snap",
            r#"---
source: src/lib.rs
expression: value
---
combined settings work
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
        "Test failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Tests that reading a text snapshot with read_binary_snapshot returns an error
#[test]
fn test_read_text_as_binary_fails() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "test_read_text_binary"
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
fn test_wrong_type() {
    // This snapshot is text, not binary
    let result = insta::read_binary_snapshot!("text_data.bin");
    assert!(result.is_err(), "Expected error when reading text as binary");
}
"#
            .to_string(),
        )
        .add_file(
            "src/snapshots/test_read_text_binary__text_data.snap",
            r#"---
source: src/lib.rs
expression: value
---
this is text, not binary
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
        "Test failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
