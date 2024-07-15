use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use insta::Snapshot;
use insta::_cargo_insta_support::{ContentError, PendingInlineSnapshot};

use crate::inline::FilePatcher;

#[derive(Clone, Copy, Debug)]
pub(crate) enum Operation {
    Accept,
    Reject,
    Skip,
}

#[derive(Debug)]
pub(crate) enum SnapshotContainerKind {
    Inline,
    External,
}

#[derive(Debug)]
pub(crate) struct PendingSnapshot {
    #[allow(dead_code)]
    id: usize,
    pub(crate) old: Option<Snapshot>,
    pub(crate) new: Snapshot,
    pub(crate) op: Operation,
    pub(crate) line: Option<u32>,
}

impl PendingSnapshot {
    pub(crate) fn summary(&self) -> String {
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
pub(crate) struct SnapshotContainer {
    snapshot_path: PathBuf,
    target_path: PathBuf,
    kind: SnapshotContainerKind,
    snapshots: Vec<PendingSnapshot>,
    patcher: Option<FilePatcher>,
}

impl SnapshotContainer {
    pub(crate) fn load(
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
                            if patcher.add_snapshot_macro(pending.line as usize) {
                                snapshots.push(PendingSnapshot {
                                    id,
                                    old: pending.old,
                                    new,
                                    op: Operation::Skip,
                                    line: Some(pending.line),
                                });
                                have_new = true;
                            } else {
                                // this is an outdated snapshot and the file changed.
                            }
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
                    fs::remove_file(&snapshot_path)
                        .map_err(|e| ContentError::FileIo(e, snapshot_path.to_path_buf()))?;
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

    pub(crate) fn target_file(&self) -> &Path {
        &self.target_path
    }

    pub(crate) fn snapshot_file(&self) -> Option<&Path> {
        match self.kind {
            SnapshotContainerKind::External => Some(&self.target_path),
            SnapshotContainerKind::Inline => None,
        }
    }

    pub(crate) fn snapshot_sort_key(&self) -> impl Ord + '_ {
        let path = self
            .snapshot_path
            .file_name()
            .and_then(|x| x.to_str())
            .unwrap_or_default();
        let mut pieces = path.rsplitn(2, '-');
        if let Some(num_suffix) = pieces.next().and_then(|x| x.parse::<i64>().ok()) {
            (pieces.next().unwrap_or(""), num_suffix)
        } else {
            (path, 0)
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.snapshots.len()
    }

    pub(crate) fn iter_snapshots(&mut self) -> impl Iterator<Item = &'_ mut PendingSnapshot> {
        self.snapshots.iter_mut()
    }

    pub(crate) fn commit(&mut self) -> Result<(), Box<dyn Error>> {
        // Try removing the snapshot file. If it fails, it's
        // likely because it another process removed it; which
        // is fine — print a message and continue.
        let try_removing_snapshot = |p| {
            fs::remove_file(p).unwrap_or_else(|_| {
                    eprintln!(
                        "Pending snapshot file at {:?} couldn't be removed. It was likely removed by another process.",
                        p
                    );
                });
        };

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
                try_removing_snapshot(&self.snapshot_path);
            }
        } else {
            // should only be one or this is weird
            for snapshot in self.snapshots.iter() {
                match snapshot.op {
                    Operation::Accept => {
                        let snapshot = Snapshot::from_file(&self.snapshot_path).map_err(|e| {
                            // If it's an IO error, pass a ContentError back so
                            // we get a slightly clearer error message
                            match e.downcast::<std::io::Error>() {
                                Ok(io_error) => Box::new(ContentError::FileIo(
                                    *io_error,
                                    self.snapshot_path.to_path_buf(),
                                )),
                                Err(other_error) => other_error,
                            }
                        })?;
                        snapshot.save(&self.target_path)?;
                        try_removing_snapshot(&self.snapshot_path);
                    }
                    Operation::Reject => {
                        try_removing_snapshot(&self.snapshot_path);
                    }
                    Operation::Skip => {}
                }
            }
        }
        Ok(())
    }
}
