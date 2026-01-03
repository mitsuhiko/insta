use std::error::Error;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use ignore::overrides::OverrideBuilder;
use ignore::{DirEntry, Walk, WalkBuilder};

use crate::container::{SnapshotContainer, TextSnapshotKind};

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

/// Finds all pending snapshots by searching `pending_root` and mapping paths to `target_root`.
///
/// # Path Structure
///
/// Pending snapshots maintain the same directory structure relative to their root.
/// For example, with a deeply nested package:
///
/// ```text
/// workspace/services/api/auth/src/snapshots/login__test.snap      <- accepted snapshot
/// pending_root/services/api/auth/src/snapshots/login__test.snap.new  <- pending snapshot
/// ```
///
/// The relative path (`services/api/auth/src/snapshots/login__test.snap`) is preserved.
/// We just swap the root prefix when mapping pending → target.
///
/// # Default Behavior
///
/// When `INSTA_PENDING_DIR` is not set, `pending_root == target_root` (both are the same
/// package-specific snapshot root). The path mapping becomes a no-op: the target is simply
/// the pending path with `.new` stripped.
///
/// # Hermetic Builds (Bazel)
///
/// When `INSTA_PENDING_DIR` is set, pending snapshots are written to a separate directory
/// (e.g., Bazel's output directory) while the source tree remains read-only. The structure
/// is preserved, so we can map back: `pending_root/relative/path` → `target_root/relative/path`.
pub(crate) fn find_pending_snapshots<'a>(
    pending_root: &'a Path,
    target_root: &'a Path,
    extensions: &'a [&'a str],
    flags: FindFlags,
) -> impl Iterator<Item = Result<SnapshotContainer, Box<dyn Error>>> + 'a {
    let pending_root_owned = pending_root.to_path_buf();
    let target_root_owned = target_root.to_path_buf();
    make_snapshot_walker(pending_root, extensions, flags)
        .filter_map(Result::ok)
        .filter_map(move |entry| {
            let fname = entry.file_name().to_string_lossy();
            let pending_path = entry.clone().into_path();

            // Map from pending_root to target_root by preserving the relative path.
            // When pending_root == target_root, this is equivalent to just stripping ".new".
            let compute_target = |new_fname: &str| -> Option<PathBuf> {
                let relative = pending_path.strip_prefix(&pending_root_owned).ok()?;
                Some(target_root_owned.join(relative).with_file_name(new_fname))
            };

            #[allow(clippy::manual_map)]
            if let Some(new_fname) = fname.strip_suffix(".new") {
                let target_path = compute_target(new_fname)?;
                Some(SnapshotContainer::load(
                    pending_path,
                    target_path,
                    TextSnapshotKind::File,
                ))
            } else if let Some(new_fname) = fname
                .strip_prefix('.')
                .and_then(|f| f.strip_suffix(".pending-snap"))
            {
                let target_path = compute_target(new_fname)?;
                Some(SnapshotContainer::load(
                    pending_path,
                    target_path,
                    TextSnapshotKind::Inline,
                ))
            } else {
                None
            }
        })
}

/// Creates a walker for snapshots & pending snapshots within a directory. The
/// walker returns snapshots ending in any of the supplied extensions, any of
/// the supplied extensions with a `.new` suffix, and `.pending-snap` files.
pub(crate) fn make_snapshot_walker(root: &Path, extensions: &[&str], flags: FindFlags) -> Walk {
    let mut builder = WalkBuilder::new(root);
    builder.standard_filters(!flags.include_ignored);
    // Disable the built-in hidden filter; we handle hidden files/dirs in our custom filter_entry
    // to allow .pending-snap files while still skipping hidden directories.
    builder.hidden(false);

    let mut override_builder = OverrideBuilder::new(root);

    extensions
        .iter()
        .flat_map(|ext| [format!("*.{ext}"), format!("*.{ext}.new")])
        .chain(std::iter::once("*.pending-snap".to_string()))
        .for_each(|pattern| {
            override_builder.add(&pattern).unwrap();
        });
    builder.overrides(override_builder.build().unwrap());

    let root_path = root.to_path_buf();
    let include_hidden = flags.include_hidden;

    builder.filter_entry(move |entry| {
        // Always skip `target` directories
        if entry.path().file_name() == Some(OsStr::new("target")) {
            return false;
        }
        // Skip nested crates (directories with Cargo.toml that aren't the search root).
        // This avoids duplicate snapshots when a package contains nested packages.
        // Not needed when walking a pending_dir (no Cargo.toml files there).
        if entry.file_type().map_or(false, |ft| ft.is_dir())
            && entry.path().join("Cargo.toml").exists()
            && entry.path() != root_path
        {
            return false;
        }
        // Skip hidden directories (unless include_hidden), but always allow files
        if !include_hidden && !entry.file_type().map_or(false, |x| x.is_file()) && is_hidden(entry)
        {
            return false;
        }

        true
    });

    builder.build()
}
