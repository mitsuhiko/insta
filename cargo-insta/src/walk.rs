use std::error::Error;
use std::ffi::OsStr;
use std::path::Path;

use ignore::overrides::OverrideBuilder;
use ignore::{DirEntry, Walk, WalkBuilder};

use crate::container::{SnapshotContainer, SnapshotKind};

#[derive(Debug, Copy, Clone)]
pub(crate) struct FindFlags {
    pub(crate) include_ignored: bool,
    pub(crate) include_hidden: bool,
}

fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with('.'))
        .unwrap_or(false)
}

/// Finds all pending snapshots
pub(crate) fn find_pending_snapshots<'a>(
    package_root: &Path,
    extensions: &'a [&'a str],
    flags: FindFlags,
) -> impl Iterator<Item = Result<SnapshotContainer, Box<dyn Error>>> + 'a {
    make_snapshot_walker(package_root, extensions, flags)
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let fname = entry.file_name().to_string_lossy();
            let path = entry.clone().into_path();

            #[allow(clippy::manual_map)]
            if let Some(new_fname) = fname.strip_suffix(".new") {
                Some(SnapshotContainer::load(
                    path.clone(),
                    path.with_file_name(new_fname),
                    SnapshotKind::File,
                ))
            } else if let Some(new_fname) = fname
                .strip_prefix('.')
                .and_then(|f| f.strip_suffix(".pending-snap"))
            {
                Some(SnapshotContainer::load(
                    path.clone(),
                    path.with_file_name(new_fname),
                    SnapshotKind::Inline,
                ))
            } else {
                None
            }
        })
}

/// Creates a walker for snapshots & pending snapshots within a package. The
/// walker returns snapshots ending in any of the supplied extensions, any of
/// the supplied extensions with a `.new` suffix, and `.pending-snap` files.
pub(crate) fn make_snapshot_walker(
    package_root: &Path,
    extensions: &[&str],
    flags: FindFlags,
) -> Walk {
    let mut builder = WalkBuilder::new(package_root);
    builder.standard_filters(!flags.include_ignored);
    if flags.include_hidden {
        builder.hidden(false);
    } else {
        // We add a custom hidden filter; if we used the standard filter we'd skip over `.pending-snap` files
        builder.filter_entry(|e| e.file_type().map_or(false, |x| x.is_file()) || !is_hidden(e));
    }

    let mut override_builder = OverrideBuilder::new(package_root);

    extensions
        .iter()
        .flat_map(|ext| [format!("*.{}", ext), format!("*.{}.new", ext)])
        .chain(std::iter::once("*.pending-snap".to_string()))
        .for_each(|pattern| {
            override_builder.add(&pattern).unwrap();
        });
    builder.overrides(override_builder.build().unwrap());

    let root_path = package_root.to_path_buf();
    // Add a custom filter to skip interior crates; otherwise we get duplicate
    // snapshots (https://github.com/mitsuhiko/insta/issues/396)
    builder.filter_entry(move |entry| {
        if entry.file_type().map_or(false, |ft| ft.is_dir())
            && entry.path().join("Cargo.toml").exists()
            && entry.path() != root_path
        {
            // Skip this directory if it contains a Cargo.toml and is not the root
            return false;
        }
        // We always want to skip `target` even if it was not excluded by
        // ignore files.
        if entry.path().file_name() == Some(OsStr::new("target")) {
            return false;
        }

        true
    });

    builder.build()
}
