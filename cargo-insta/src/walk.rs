use std::collections::HashSet;
use std::error::Error;
use std::ffi::OsStr;
use std::path::Path;

use ignore::overrides::OverrideBuilder;
use ignore::{DirEntry, Walk, WalkBuilder};

use crate::cargo::Package;
use crate::container::{SnapshotContainer, SnapshotContainerKind};

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

/// Finds all snapshots
pub(crate) fn find_snapshots<'a>(
    root: &Path,
    extensions: &'a [&'a str],
    flags: FindFlags,
) -> impl Iterator<Item = Result<SnapshotContainer, Box<dyn Error>>> + 'a {
    make_snapshot_walker(root, extensions, flags)
        .filter_map(|e| e.ok())
        .filter_map(move |e| {
            let fname = e.file_name().to_string_lossy();
            if fname.ends_with(".new") {
                let new_path = e.into_path();
                let mut old_path = new_path.clone();
                old_path.set_extension("");
                Some(SnapshotContainer::load(
                    new_path,
                    old_path,
                    SnapshotContainerKind::External,
                ))
            } else if fname.starts_with('.') && fname.ends_with(".pending-snap") {
                let mut target_path = e.path().to_path_buf();
                target_path.set_file_name(&fname[1..fname.len() - 13]);
                Some(SnapshotContainer::load(
                    e.path().to_path_buf(),
                    target_path,
                    SnapshotContainerKind::Inline,
                ))
            } else {
                None
            }
        })
}

/// Creates a walker for snapshots.
pub(crate) fn make_snapshot_walker(path: &Path, extensions: &[&str], flags: FindFlags) -> Walk {
    let mut builder = WalkBuilder::new(path);
    builder.standard_filters(!flags.include_ignored);
    if flags.include_hidden {
        builder.hidden(false);
    } else {
        builder.filter_entry(|e| e.file_type().map_or(false, |x| x.is_file()) || !is_hidden(e));
    }

    let mut override_builder = OverrideBuilder::new(path);
    override_builder
        .add(".*.pending-snap")
        .unwrap()
        .add("*.snap.new")
        .unwrap();

    for ext in extensions {
        override_builder.add(&format!("*.{}.new", ext)).unwrap();
    }

    builder.overrides(override_builder.build().unwrap());
    builder.build()
}

/// A walker that is used by the snapshot deletion code.
///
/// This really should be using the same logic as the main snapshot walker but today is is not.
pub(crate) fn make_deletion_walker(
    workspace_root: &Path,
    known_packages: Option<&[Package]>,
    selected_package: Option<&str>,
) -> Walk {
    let roots: HashSet<_> = if let Some(packages) = known_packages {
        packages
            .iter()
            .filter_map(|x| {
                // filter out packages we did not ask for.
                if let Some(only_package) = selected_package {
                    if x.name != only_package {
                        return None;
                    }
                }
                x.manifest_path.parent().unwrap().canonicalize().ok()
            })
            .collect()
    } else {
        Some(workspace_root.to_path_buf()).into_iter().collect()
    };

    WalkBuilder::new(workspace_root)
        .filter_entry(move |entry| {
            // we only filter down for directories
            if !entry.file_type().map_or(false, |x| x.is_dir()) {
                return true;
            }

            let canonicalized = match entry.path().canonicalize() {
                Ok(path) => path,
                Err(_) => return true,
            };

            // We always want to skip target even if it was not excluded by
            // ignore files.
            if entry.path().file_name() == Some(OsStr::new("target"))
                && roots.contains(canonicalized.parent().unwrap())
            {
                return false;
            }

            // do not enter crates which are not in the list of known roots
            // of the workspace.
            if !roots.contains(&canonicalized)
                && entry
                    .path()
                    .join("Cargo.toml")
                    .metadata()
                    .map_or(false, |x| x.is_file())
            {
                return false;
            }

            true
        })
        .build()
}
