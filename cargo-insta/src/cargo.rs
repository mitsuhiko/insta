use std::path::{Path, PathBuf};

pub(crate) use cargo_metadata::Package;

/// Find snapshot roots within a package
// We need this because paths are not always conventional — for example cargo
// can reference artifacts that are outside of the package root.
pub(crate) fn find_snapshot_roots(package: &Package) -> Vec<PathBuf> {
    let mut roots = std::collections::HashSet::new();

    // the manifest path's parent is always a snapshot container.  For
    // a rationale see GH-70.  But generally a user would expect to be
    // able to put a snapshot into foo/snapshots instead of foo/src/snapshots.
    if let Some(manifest) = package.manifest_path.parent() {
        roots.insert(manifest.as_std_path().to_path_buf());
    }

    // additionally check all targets.
    for target in &package.targets {
        // custom build scripts we can safely skip over.
        if target.kind.iter().any(|kind| kind == "custom-build") {
            continue;
        }

        // this gives us the containing source folder.  Typically this is
        // something like crate/src.
        let root = target.src_path.parent().unwrap().as_std_path();
        roots.insert(root.to_path_buf());
    }

    // Convert HashSet back to Vec for the rest of the function
    let roots: Vec<_> = roots.into_iter().collect();

    // TODO: I think this root reduction is duplicative over the logic in
    // `make_snapshot_walker`; could try removing.

    // Reduce roots to avoid traversing into paths twice.  If we have both
    // `/foo` and `/foo/bar` as roots we only keep `/foo`.  Otherwise we
    // would encounter the same snapshot twice — e.g. the existence of a
    // build script can cause the same path to be determined twice.  (GH-15)
    //
    // The comparison is done on canonicalized paths so that symlinks and
    // Windows path quirks (8.3 short names, etc.) don't defeat it, but we
    // return the *original* paths.  Canonicalized paths on Windows carry a
    // `\\?\` verbatim prefix that would otherwise leak into snapshot keys
    // and other user-facing output.  (GH-902)
    fn canonical(path: &Path) -> PathBuf {
        path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
    }
    let roots: Vec<(PathBuf, PathBuf)> = roots
        .into_iter()
        .map(|root| {
            let canonical = canonical(&root);
            (root, canonical)
        })
        .collect();
    roots
        .iter()
        .filter(|(_, canonical)| {
            !roots
                .iter()
                .any(|(_, other)| canonical != other && canonical.starts_with(other))
        })
        .map(|(root, _)| root.clone())
        .collect()
}
