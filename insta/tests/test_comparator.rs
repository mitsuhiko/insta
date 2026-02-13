//! Provides end-to-end tests of an example custom [`Comparator`]
//! implementation. If you're interested in writing macros with custom snapshot
//! comparison behavior, consult these examples.

use insta::{assert_snapshot, with_settings, Comparator, Snapshot};

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

    fn dyn_clone(&self) -> Box<dyn Comparator> {
        Box::new(WhitespaceInsensitiveComparator)
    }
}

#[test]
fn custom_comparator_matches() {
    let comparator = Box::new(WhitespaceInsensitiveComparator);

    // "hello world" matches "hello    world" when whitespace is ignored
    with_settings!({comparator => comparator}, {
        assert_snapshot!("hello world", @"hello    world");
    });
}

macro_rules! assert_whitespace_insensitive {
    ($($body:tt)*) => {
        ::insta::with_settings!(
            {comparator => Box::new(crate::WhitespaceInsensitiveComparator)},
            {
                ::insta::assert_snapshot!($($body)*);
            });
    };
}

#[test]
fn custom_macro_passes() {
    assert_whitespace_insensitive!("hello world", @"hello    world");
}
