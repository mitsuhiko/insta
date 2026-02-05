//! Provides end-to-end tests of an example custom [`Comparator`]
//! implementation. If you're interested in writing macros with custom snapshot
//! comparison behavior, consult these examples.

use insta::comparator::Comparator;
use insta::internals::{SnapshotContents, TextSnapshotContents};
use insta::{assert_snapshot, with_settings, Snapshot, TextSnapshotKind};

/// Passes all comparisons if `reference` is just an inline snapshot with the
/// text "pass".
struct MyComparator;

impl Comparator for MyComparator {
    fn matches(&self, reference: &Snapshot, _test: &Snapshot) -> bool {
        reference.contents()
            == &SnapshotContents::Text(TextSnapshotContents::new(
                String::from("pass"),
                TextSnapshotKind::Inline,
            ))
    }

    fn dyn_clone(&self) -> Box<dyn Comparator> {
        Box::new(MyComparator)
    }
}

#[test]
fn custom_comparator_matches() {
    let comparator = Box::new(MyComparator);

    with_settings!({comparator => comparator}, {
        assert_snapshot!("reference", @"pass");
    });
}

#[test]
#[should_panic(expected = "snapshot assertion for 'custom_comparator_fails' failed")]
fn custom_comparator_fails() {
    let comparator = Box::new(MyComparator);

    with_settings!({comparator => comparator}, {
        assert_snapshot!("reference", @"fail");
    });
}

macro_rules! assert_my_comparator {
    ($($body:tt)*) => {
        ::insta::with_settings!(
            {comparator => Box::new(crate::MyComparator)},
            {
                ::insta::assert_snapshot!($($body)*);
            });
    };
}

#[test]
fn custom_macro_passes() {
    assert_my_comparator!("reference", @"pass");
}

#[test]
#[should_panic(expected = "snapshot assertion for 'custom_macro_fails' failed")]
fn custom_macro_fails() {
    assert_my_comparator!("reference", @"fail");
}
