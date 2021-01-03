use std::collections::HashSet;
use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

use ignore::overrides::OverrideBuilder;
use ignore::{DirEntry, WalkBuilder};
use insta::{PendingInlineSnapshot, Snapshot};
use serde::Deserialize;

use crate::inline::FilePatcher;
use crate::utils::err_msg;

#[derive(Deserialize, Clone, Debug)]
pub struct Target {
    src_path: PathBuf,
    kind: HashSet<String>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Package {
    name: String,
    version: String,
    id: String,
    manifest_path: PathBuf,
    targets: Vec<Target>,
}

#[derive(Deserialize, Debug)]
pub struct Metadata {
    packages: Vec<Package>,
    workspace_members: Vec<String>,
    workspace_root: String,
}

#[derive(Clone, Copy, Debug)]
pub enum Operation {
    Accept,
    Reject,
    Skip,
}

impl Metadata {
    pub fn workspace_root(&self) -> &Path {
        Path::new(&self.workspace_root)
    }
}

#[derive(Deserialize, Debug)]
struct ProjectLocation {
    root: PathBuf,
}

#[derive(Debug)]
pub enum SnapshotContainerKind {
    Inline,
    External,
}

#[derive(Debug)]
pub struct PendingSnapshot {
    pub id: usize,
    pub old: Option<Snapshot>,
    pub new: Snapshot,
    pub op: Operation,
    pub line: Option<u32>,
}

impl PendingSnapshot {
    pub fn summary(&self) -> String {
        use std::fmt::Write;
        let mut rv = String::new();
        if let Some(source) = self.new.metadata().source() {
            write!(&mut rv, "{}", source).unwrap();
        }
        if let Some(line) = self.line {
            write!(&mut rv, ":{}", line).unwrap();
        }
        if let Some(name) = self.new.snapshot_name() {
            write!(&mut rv, " ({})", name).unwrap();
        }
        rv
    }
}

#[derive(Debug)]
pub struct SnapshotContainer {
    snapshot_path: PathBuf,
    target_path: PathBuf,
    kind: SnapshotContainerKind,
    snapshots: Vec<PendingSnapshot>,
    patcher: Option<FilePatcher>,
}

impl SnapshotContainer {
    fn load(
        snapshot_path: PathBuf,
        target_path: PathBuf,
        kind: SnapshotContainerKind,
    ) -> Result<SnapshotContainer, Box<dyn Error>> {
        let mut snapshots = Vec::new();
        let patcher = match kind {
            SnapshotContainerKind::External => {
                let old = if fs::metadata(&target_path).is_err() {
                    None
                } else {
                    Some(Snapshot::from_file(&target_path)?)
                };
                let new = Snapshot::from_file(&snapshot_path)?;
                snapshots.push(PendingSnapshot {
                    id: 0,
                    old,
                    new,
                    op: Operation::Skip,
                    line: None,
                });
                None
            }
            SnapshotContainerKind::Inline => {
                let mut pending_vec = PendingInlineSnapshot::load_batch(&snapshot_path)?;
                let mut have_new = false;

                let rv = if fs::metadata(&target_path).is_ok() {
                    let mut patcher = FilePatcher::open(&target_path)?;
                    pending_vec.sort_by_key(|pending| pending.line);
                    for (id, pending) in pending_vec.into_iter().enumerate() {
                        if let Some(new) = pending.new {
                            snapshots.push(PendingSnapshot {
                                id,
                                old: pending.old,
                                new,
                                op: Operation::Skip,
                                line: Some(pending.line),
                            });
                            patcher.add_snapshot_macro(pending.line as usize);
                            have_new = true;
                        }
                    }
                    Some(patcher)
                } else {
                    None
                };

                // if we don't actually have any new pending we better delete the file.
                // this can happen if the test code left a stale snapshot behind.
                // The runtime code will issue something like this:
                //   PendingInlineSnapshot::new(None, None, line).save(pending_snapshots)?;
                if !have_new {
                    fs::remove_file(&snapshot_path)?;
                }

                rv
            }
        };

        Ok(SnapshotContainer {
            snapshot_path,
            target_path,
            kind,
            snapshots,
            patcher,
        })
    }

    pub fn target_file(&self) -> &Path {
        &self.target_path
    }

    pub fn snapshot_file(&self) -> Option<&Path> {
        match self.kind {
            SnapshotContainerKind::External => Some(&self.target_path),
            SnapshotContainerKind::Inline => None,
        }
    }

    pub fn len(&self) -> usize {
        self.snapshots.len()
    }

    pub fn iter_snapshots(&mut self) -> impl Iterator<Item = &'_ mut PendingSnapshot> {
        self.snapshots.iter_mut()
    }

    pub fn commit(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(ref mut patcher) = self.patcher {
            let mut new_pending = vec![];
            let mut did_accept = false;
            let mut did_skip = false;

            for (idx, snapshot) in self.snapshots.iter().enumerate() {
                match snapshot.op {
                    Operation::Accept => {
                        patcher.set_new_content(idx, snapshot.new.contents());
                        did_accept = true;
                    }
                    Operation::Reject => {}
                    Operation::Skip => {
                        new_pending.push(PendingInlineSnapshot::new(
                            Some(snapshot.new.clone()),
                            snapshot.old.clone(),
                            patcher.get_new_line(idx) as u32,
                        ));
                        did_skip = true;
                    }
                }
            }

            if did_accept {
                patcher.save()?;
            }
            if did_skip {
                PendingInlineSnapshot::save_batch(&self.snapshot_path, &new_pending)?;
            } else {
                fs::remove_file(&self.snapshot_path)?;
            }
        } else {
            // should only be one or this is weird
            for snapshot in self.snapshots.iter() {
                match snapshot.op {
                    Operation::Accept => {
                        fs::rename(&self.snapshot_path, &self.target_path)?;
                    }
                    Operation::Reject => {
                        fs::remove_file(&self.snapshot_path)?;
                    }
                    Operation::Skip => {}
                }
            }
        }
        Ok(())
    }
}

fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with('.'))
        .unwrap_or(false)
}

pub fn find_snapshots<'a>(
    root: PathBuf,
    extensions: &'a [&'a str],
    no_ignore: bool,
) -> impl Iterator<Item = Result<SnapshotContainer, Box<dyn Error>>> + 'a {
    WalkBuilder::new(root.clone())
        .hidden(false)
        .standard_filters(!no_ignore)
        .overrides(
            // make sure pending snaps are never ignored
            OverrideBuilder::new(&root)
                .add("**/.*.pending-snap")
                .unwrap()
                .add("**/*.snap.new")
                .unwrap()
                .build()
                .unwrap(),
        )
        .filter_entry(|e| e.file_type().map_or(false, |x| x.is_file()) || !is_hidden(e))
        .build()
        .filter_map(|e| e.ok())
        .filter_map(move |e| {
            let fname = e.file_name().to_string_lossy();
            if fname.ends_with(".new")
                && extensions.contains(&fname.rsplit('.').nth(1).unwrap_or(""))
            {
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

impl Package {
    pub fn manifest_path(&self) -> &Path {
        &self.manifest_path
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn iter_snapshot_containers<'a>(
        &self,
        extensions: &'a [&'a str],
        no_ignore: bool,
    ) -> impl Iterator<Item = Result<SnapshotContainer, Box<dyn Error>>> + 'a {
        let mut roots = Vec::new();

        // the manifest path's parent is always a snapshot container.  For
        // a rationale see GH-70.  But generally a user would expect to be
        // able to put a snapshot into foo/snapshots instead of foo/src/snapshots.
        if let Some(manifest) = self.manifest_path.parent() {
            roots.push(manifest.to_path_buf());
        }

        // additionally check all targets.
        for target in &self.targets {
            // custom build scripts we can safely skip over.  In the past this
            // caused issues with duplicate paths but that's resolved in other
            // ways now.  We do not want to pick up snapshots in such places
            // though.
            if target.kind.contains("custom-build") {
                continue;
            }

            // this gives us the containing source folder.  Typically this is
            // something like crate/src.
            let root = target.src_path.parent().unwrap();
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
            if !reduced_roots.iter().any(|x| root.starts_with(&x)) {
                reduced_roots.push(root);
            }
        }

        reduced_roots
            .into_iter()
            .flat_map(move |root| find_snapshots(root, extensions, no_ignore))
    }
}

pub fn get_cargo() -> String {
    env::var("CARGO")
        .ok()
        .unwrap_or_else(|| "cargo".to_string())
}

pub fn get_package_metadata(manifest_path: Option<&Path>) -> Result<Metadata, Box<dyn Error>> {
    let mut cmd = process::Command::new(get_cargo());
    cmd.arg("metadata")
        .arg("--no-deps")
        .arg("--format-version=1");
    if let Some(manifest_path) = manifest_path {
        if !fs::metadata(manifest_path)
            .ok()
            .map_or(false, |x| x.is_file())
        {
            return Err(err_msg(
                "the manifest-path must be a path to a Cargo.toml file",
            ));
        }
        cmd.arg("--manifest-path").arg(manifest_path.as_os_str());
    }
    let output = cmd.output()?;
    if !output.status.success() {
        let msg = String::from_utf8_lossy(&output.stderr);
        return Err(err_msg(format!(
            "cargo erroried getting metadata: {}",
            msg.trim()
        )));
    }
    Ok(serde_json::from_slice(&output.stdout)?)
}

fn get_default_manifest() -> Result<Option<PathBuf>, Box<dyn Error>> {
    let output = process::Command::new(get_cargo())
        .arg("locate-project")
        .output()?;
    if output.status.success() {
        let loc: ProjectLocation = serde_json::from_slice(&output.stdout)?;
        Ok(Some(loc.root))
    } else {
        Ok(None)
    }
}

fn find_all_packages(metadata: &Metadata) -> Vec<Package> {
    metadata
        .packages
        .iter()
        .filter_map(|package| {
            if metadata.workspace_members.contains(&package.id) {
                Some(package.clone())
            } else {
                None
            }
        })
        .collect()
}

pub fn find_packages(metadata: &Metadata, all: bool) -> Result<Vec<Package>, Box<dyn Error>> {
    if all {
        Ok(find_all_packages(metadata))
    } else {
        let default_manifest = get_default_manifest()?
            .ok_or_else(|| {
                err_msg(
                    "Could not find `Cargo.toml` in the current folder or any parent directory.",
                )
            })?
            .canonicalize()?;
        let mut rv = vec![];
        for package in &metadata.packages {
            if package.manifest_path.canonicalize()? == default_manifest {
                rv.push(package.clone());
            }
        }
        if rv.is_empty() {
            // if we don't find anything we're in a workspace root that has no
            // root member in which case --all is implied.
            Ok(find_all_packages(metadata))
        } else {
            Ok(rv)
        }
    }
}
