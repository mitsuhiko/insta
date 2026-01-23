//! Provides the [`Comparator`] trait, which provides a mechanism for specifying
//! how [`Snapshot`] data should be compared.

use crate::env::ToolConfig;
use crate::snapshot::{Snapshot, SnapshotContents, TextSnapshotKind};

/// Allows specific behavior to be invoked when [`Snapshot`]s are compared.
///
/// This is intended for use in macros that pass an implementation to
/// [`insta::runtime::assert_snapshot`]. You probably don't need to call these
/// methods directly.
pub trait Comparator {
    /// Returns `true` if and only if `reference` and `test` match.
    ///
    /// Implementations should panic to report unrecoverable errors (e.g.,
    /// snapshot data is text data when binary was expected, or binary snapshot
    /// data is in the wrong format).
    fn matches(&self, config: &ToolConfig, reference: &Snapshot, test: &Snapshot) -> bool;
}

struct DefaultComparator;

impl DefaultComparator {
    fn contents_match(&self, reference: &Snapshot, test: &Snapshot) -> bool {
        reference.contents() == test.contents()
            // For binary snapshots the extension also need to be the same:
            && reference.metadata().snapshot_kind == test.metadata().snapshot_kind
    }
}

impl Comparator for DefaultComparator {
    fn matches(&self, config: &ToolConfig, reference: &Snapshot, test: &Snapshot) -> bool {
        if config.require_full_match() {
            // Both the exact snapshot contents and the persisted metadata match another snapshot's.
            match (reference.contents(), test.contents()) {
                (SnapshotContents::Text(ref_contents), SnapshotContents::Text(test_contents)) => {
                    // Note that we previously would match the exact values of the
                    // unnormalized text. But that's too strict — it means we can
                    // never match a snapshot that has leading/trailing whitespace.
                    // So instead we check it matches on the latest format.
                    // Generally those should be the same — latest should be doing
                    // the minimum normalization; if they diverge we could update
                    // this to be stricter.
                    //
                    // (I think to do this perfectly, we'd want to match the
                    // _reference_ value unnormalized, but the _generated_ value
                    // normalized. That way, we can get the But at the moment we
                    // don't distinguish between which is which in our data
                    // structures.)
                    let contents_match_exact = ref_contents.matches_latest(test_contents);
                    match ref_contents.kind {
                        TextSnapshotKind::File => {
                            reference.metadata().trim_for_persistence()
                                == test.metadata().trim_for_persistence()
                                && contents_match_exact
                        }
                        TextSnapshotKind::Inline => contents_match_exact,
                    }
                }
                _ => self.contents_match(reference, test),
            }
        } else {
            self.contents_match(reference, test)
        }
    }
}

/// Provides default comparison semantics for [`Snapshot`]s. Binary snapshots
/// are compared on the basis of their contents (including file extension). Text
/// snapshots are compared on the basis of their deserialized representation.
///
/// Text snapshot comparison respects the [`ToolConfig::require_full_match`]
/// setting.
pub const DEFAULT_COMPARATOR: &'static dyn Comparator = &DefaultComparator;

#[cfg(test)]
mod test {
    use super::DEFAULT_COMPARATOR;

    use crate::env::ToolConfig;
    use crate::snapshot::{
        MetaData, Snapshot, SnapshotContents, TextSnapshotContents, TextSnapshotKind,
    };

    const TEXT: &'static str =
        "The sky above the port was the color of a television, tuned to a dead channel.";

    #[test]
    fn default_comparator_default_match() {
        let mut config = ToolConfig::default();
        let a = Snapshot::from_components(
            String::from("test"),
            None,
            MetaData::default(),
            SnapshotContents::Text(TextSnapshotContents::new(
                String::from(TEXT),
                TextSnapshotKind::Inline,
            )),
        );
        let b = a.clone();
        assert!(DEFAULT_COMPARATOR.matches(&config, &a, &b));
        config.set_require_full_match(true);
        assert!(DEFAULT_COMPARATOR.matches(&config, &a, &b));
    }

    #[test]
    fn default_comparator_exact_match() {
        let mut config = ToolConfig::default();
        let a = Snapshot::from_components(
            String::from("test"),
            None,
            MetaData::default(),
            SnapshotContents::Text(TextSnapshotContents::new(
                String::from(TEXT),
                TextSnapshotKind::File,
            )),
        );
        let mut b = Snapshot::from_components(
            String::from("test"),
            None,
            MetaData::default(),
            SnapshotContents::Text(TextSnapshotContents::new(
                String::from(TEXT),
                TextSnapshotKind::Inline,
            )),
        );
        b.metadata.description = Some(String::from("wintermute")); // Differs from None in a.

        // Comparing contents alone passes.
        assert!(DEFAULT_COMPARATOR.matches(&config, &a, &b));

        config.set_require_full_match(true);
        // Comparing contents alone still passes.
        assert!(DEFAULT_COMPARATOR.matches(&config, &a, &a));
        // Comparing snapshots with differing metadata fails.
        assert!(!DEFAULT_COMPARATOR.matches(&config, &a, &b));
    }
}
