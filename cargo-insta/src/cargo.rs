use std::error::Error;
use std::path::{Path, PathBuf};

pub(crate) use cargo_metadata::{Metadata, Package};

pub(crate) fn find_snapshot_roots(package: &Package) -> Vec<PathBuf> {
    let mut roots = Vec::new();

    // the manifest path's parent is always a snapshot container.  For
    // a rationale see GH-70.  But generally a user would expect to be
    // able to put a snapshot into foo/snapshots instead of foo/src/snapshots.
    if let Some(manifest) = package.manifest_path.parent() {
        roots.push(manifest.as_std_path().to_path_buf());
    }

    // additionally check all targets.
    for target in &package.targets {
        // custom build scripts we can safely skip over.  In the past this
        // caused issues with duplicate paths but that's resolved in other
        // ways now.  We do not want to pick up snapshots in such places
        // though.
        if target.kind.iter().any(|kind| kind == "custom-build") {
            continue;
        }

        // this gives us the containing source folder.  Typically this is
        // something like crate/src.
        let root = target.src_path.parent().unwrap().as_std_path();
        roots.push(root.to_path_buf());
    }

    // reduce roots to avoid traversing into paths twice.  If we have both
    // /foo and /foo/bar as roots we would only walk into /foo.  Otherwise
    // we would encounter paths twice.  If we don't skip them here we run
    // into issues where the existence of a build script causes a snapshot
    // to be picked up twice since the same path is determined.  (GH-15)
    roots.sort_by_key(|x| x.as_os_str().len());
    let mut reduced_roots = vec![];
    for root in roots {
        if !reduced_roots.iter().any(|x| root.starts_with(x)) {
            reduced_roots.push(root);
        }
    }

    reduced_roots
}

pub(crate) fn get_metadata(
    manifest_path: Option<&Path>,
    all: bool,
) -> Result<Metadata, Box<dyn Error>> {
    let mut cmd = cargo_metadata::MetadataCommand::new();
    if let Some(manifest_path) = manifest_path {
        cmd.manifest_path(manifest_path);
    }
    if all {
        cmd.no_deps();
    }
    let mut metadata = cmd.exec()?;
    let Metadata {
        packages,
        workspace_members,
        resolve,
        ..
    } = &mut metadata;
    match resolve
        .as_ref()
        .and_then(|cargo_metadata::Resolve { root, .. }| root.as_ref())
    {
        Some(root) => packages.retain(|Package { id, .. }| id == root),
        None => {
            packages.retain(|Package { id, .. }| workspace_members.contains(id));
        }
    }
    Ok(metadata)
}
