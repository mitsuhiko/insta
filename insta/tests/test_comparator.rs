//! Provides end-to-end tests of custom [`Comparator`] implementations.
//!
//! If you're interested in writing macros with custom snapshot comparison
//! behavior, consult these examples.

use insta::{assert_snapshot, with_settings, Comparator, Snapshot};

// --- Custom comparator: whitespace insensitive ---

/// A comparator that ignores whitespace differences.
struct WhitespaceInsensitiveComparator;

impl Comparator for WhitespaceInsensitiveComparator {
    fn matches(&self, reference: &Snapshot, test: &Snapshot) -> bool {
        match (reference.as_text(), test.as_text()) {
            (Some(a), Some(b)) => {
                let a_normalized: String = a.to_string().split_whitespace().collect();
                let b_normalized: String = b.to_string().split_whitespace().collect();
                a_normalized == b_normalized
            }
            _ => false,
        }
    }
}

#[test]
fn whitespace_insensitive_comparator() {
    // The value has single spaces, reference has multiple - custom comparator should match
    let value = "hello world";
    with_settings!({comparator => WhitespaceInsensitiveComparator}, {
        assert_snapshot!(value, @"hello    world");
    });
}

// --- Custom comparator: always passes ---

/// A comparator that always passes (for testing purposes).
struct AlwaysPassComparator;

impl Comparator for AlwaysPassComparator {
    fn matches(&self, _reference: &Snapshot, _test: &Snapshot) -> bool {
        true
    }
}

#[test]
fn always_pass_comparator() {
    with_settings!({comparator => AlwaysPassComparator}, {
        // Any value matches any reference with this comparator
        assert_snapshot!("anything at all", @"completely different");
    });
}

// --- Custom comparator: prefix matching ---

/// A comparator that passes if the test value starts with the reference.
struct PrefixComparator;

impl Comparator for PrefixComparator {
    fn matches(&self, reference: &Snapshot, test: &Snapshot) -> bool {
        match (reference.as_text(), test.as_text()) {
            (Some(ref_text), Some(test_text)) => {
                test_text.to_string().starts_with(&ref_text.to_string())
            }
            _ => false,
        }
    }
}

#[test]
fn prefix_comparator() {
    with_settings!({comparator => PrefixComparator}, {
        // "hello world" starts with "hello"
        assert_snapshot!("hello world", @"hello");
    });
}
