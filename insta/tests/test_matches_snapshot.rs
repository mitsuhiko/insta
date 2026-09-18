use insta::{assert_snapshot, matches_snapshot};
use std::path::PathBuf;

fn new_snapshot_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/snapshots")
        .join(format!("test_matches_snapshot__{name}.snap.new"))
}

#[test]
fn test_matches_snapshot_matching_value() {
    // Establish the committed reference.
    assert_snapshot!("matches_snapshot_matching_value", "hello world");
    // A matching value reports `Ok(true)`.
    assert!(matches_snapshot!("matches_snapshot_matching_value", "hello world").unwrap());
}

#[test]
fn test_matches_snapshot_mismatching_value() {
    // Establish the committed reference.
    assert_snapshot!("matches_snapshot_mismatching_value", "hello world");
    // A non-matching value reports `Ok(false)` ...
    assert!(!matches_snapshot!("matches_snapshot_mismatching_value", "goodbye world").unwrap());
    // ... and crucially writes NO pending snapshot.
    let new_path = new_snapshot_path("matches_snapshot_mismatching_value");
    assert!(
        !new_path.exists(),
        "matches_snapshot! must not write a .snap.new; found {}",
        new_path.display(),
    );
}

#[test]
fn test_matches_snapshot_missing_reference() {
    // No reference exists for this name yet → `Ok(false)`, not an error, and
    // no pending snapshot written.
    let new_path = new_snapshot_path("matches_snapshot_missing_reference");
    let _ = std::fs::remove_file(&new_path);
    assert!(!matches_snapshot!("matches_snapshot_missing_reference", "anything").unwrap());
    assert!(!new_path.exists());
}

#[cfg(feature = "filters")]
#[test]
fn test_matches_snapshot_applies_filters() {
    let mut settings = insta::Settings::clone_current();
    settings.add_filter(r"PID-\d+", "PID-<masked>");
    let _guard = settings.bind_to_scope();

    // Reference stored with the filter applied → body is `pid=PID-<masked>`.
    assert_snapshot!("matches_snapshot_applies_filters", "pid=PID-12345");
    // A raw value carrying a *different* PID still matches, because the bound
    // filter masks it the same way `assert_snapshot!` would.
    assert!(matches_snapshot!("matches_snapshot_applies_filters", "pid=PID-99999").unwrap());
    let new_path = new_snapshot_path("matches_snapshot_applies_filters");
    assert!(!new_path.exists());
}

#[test]
fn test_matches_snapshot_inline() {
    // Inline snapshot form — the same calling forms as `assert_snapshot!`
    // (inline `@"…"` here) are now supported via the shared macro machinery.
    assert!(matches_snapshot!("hello world", @"hello world").unwrap());
    assert!(!matches_snapshot!("goodbye world", @"hello world").unwrap());
}

#[test]
fn test_matches_snapshot_inline_in_loop() {
    // Unlike `assert_snapshot!`, inline `matches_snapshot!` skips
    // inline-duplicate detection, so it is safe to call inside a loop
    // (the primary polling use case) without panicking.
    for _ in 0..3 {
        assert!(matches_snapshot!("hello world", @"hello world").unwrap());
        assert!(!matches_snapshot!("goodbye world", @"hello world").unwrap());
    }
}
