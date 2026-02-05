//! Functional tests for custom [`Comparator`] implementations.

use crate::TestFiles;

/// Test that a custom comparator can override default matching behavior.
#[test]
fn test_custom_comparator_inline() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "test_custom_comparator_inline"
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
use insta::comparator::Comparator;
use insta::internals::SnapshotContents;
use insta::{Snapshot, with_settings, assert_snapshot};

/// A comparator that ignores whitespace differences.
struct WhitespaceInsensitiveComparator;

impl Comparator for WhitespaceInsensitiveComparator {
    fn matches(&self, reference: &Snapshot, test: &Snapshot) -> bool {
        match (reference.contents(), test.contents()) {
            (SnapshotContents::Text(a), SnapshotContents::Text(b)) => {
                let a_normalized: String = a.to_string().split_whitespace().collect();
                let b_normalized: String = b.to_string().split_whitespace().collect();
                a_normalized == b_normalized
            }
            _ => false,
        }
    }
}

#[test]
fn test_whitespace_insensitive() {
    // The value has single spaces, reference has multiple - custom comparator should match
    let value = "hello world";
    with_settings!({comparator => WhitespaceInsensitiveComparator}, {
        assert_snapshot!(value, @"hello    world");
    });
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

/// Test that a custom comparator works with file snapshots.
#[test]
fn test_custom_comparator_file_snapshot() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "test_custom_comparator_file"
version = "0.1.0"
edition = "2021"

[dependencies]
insta = { path = '$PROJECT_PATH' }
"#
            .to_string(),
        )
        .add_file(
            "src/snapshots/test_custom_comparator_file__file_snapshot.snap",
            r#"---
source: src/lib.rs
expression: value
---
hello    world
"#
            .to_string(),
        )
        .add_file(
            "src/lib.rs",
            r#"
use insta::comparator::Comparator;
use insta::internals::SnapshotContents;
use insta::{Snapshot, with_settings, assert_snapshot};

/// A comparator that ignores whitespace differences.
struct WhitespaceInsensitiveComparator;

impl Comparator for WhitespaceInsensitiveComparator {
    fn matches(&self, reference: &Snapshot, test: &Snapshot) -> bool {
        match (reference.contents(), test.contents()) {
            (SnapshotContents::Text(a), SnapshotContents::Text(b)) => {
                let a_normalized: String = a.to_string().split_whitespace().collect();
                let b_normalized: String = b.to_string().split_whitespace().collect();
                a_normalized == b_normalized
            }
            _ => false,
        }
    }
}

#[test]
fn test_file_snapshot() {
    // The value has single spaces, stored snapshot has multiple - should match
    let value = "hello world";
    with_settings!({comparator => WhitespaceInsensitiveComparator}, {
        assert_snapshot!("file_snapshot", value);
    });
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

/// Test that matches_fully is called when INSTA_REQUIRE_FULL_MATCH is set.
#[test]
fn test_matches_fully_with_env_var() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "test_matches_fully"
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
use std::sync::atomic::{AtomicBool, Ordering};
use insta::comparator::Comparator;
use insta::{Snapshot, with_settings, assert_snapshot};

static MATCHES_FULLY_CALLED: AtomicBool = AtomicBool::new(false);

struct TrackingComparator;

impl Comparator for TrackingComparator {
    fn matches(&self, _reference: &Snapshot, _test: &Snapshot) -> bool {
        true
    }

    fn matches_fully(&self, _reference: &Snapshot, _test: &Snapshot) -> bool {
        MATCHES_FULLY_CALLED.store(true, Ordering::SeqCst);
        true
    }
}

#[test]
fn test_tracking() {
    with_settings!({comparator => TrackingComparator}, {
        assert_snapshot!("value", @"value");
    });

    // When INSTA_REQUIRE_FULL_MATCH=1 is set, matches_fully should be called
    assert!(MATCHES_FULLY_CALLED.load(Ordering::SeqCst), "matches_fully was not called");
}
"#
            .to_string(),
        )
        .create_project();

    let output = test_project
        .insta_cmd()
        .args(["test", "--", "--nocapture"])
        .env("INSTA_REQUIRE_FULL_MATCH", "1")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "Test failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Test that comparator setting is inherited in nested with_settings! blocks.
#[test]
fn test_comparator_inheritance() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "test_comparator_inheritance"
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
use insta::comparator::Comparator;
use insta::{Snapshot, with_settings, assert_snapshot};

/// Always passes.
struct AlwaysPassComparator;

impl Comparator for AlwaysPassComparator {
    fn matches(&self, _reference: &Snapshot, _test: &Snapshot) -> bool {
        true
    }
}

#[test]
fn test_nested_settings() {
    with_settings!({comparator => AlwaysPassComparator}, {
        // Outer block has custom comparator
        assert_snapshot!("outer", @"anything");

        with_settings!({description => "inner block"}, {
            // Inner block should inherit the comparator
            assert_snapshot!("inner", @"different content");
        });
    });
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
