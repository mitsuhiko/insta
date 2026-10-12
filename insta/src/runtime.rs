use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::str;
use std::sync::{Arc, Mutex};
use std::{borrow::Cow, env};

use crate::settings::Settings;
use crate::snapshot::{
    MetaData, PendingInlineSnapshot, Snapshot, SnapshotContents, SnapshotKind, TextSnapshotContents,
};
use crate::utils::{path_to_storage, style};
use crate::{env::get_tool_config, output::SnapshotPrinter};
use crate::{
    env::{
        memoize_snapshot_file, pending_snapshot_path, snapshot_update_behavior, OutputBehavior,
        SnapshotUpdateBehavior, ToolConfig,
    },
    snapshot::TextSnapshotKind,
};

use once_cell::sync::Lazy;

static TEST_NAME_COUNTERS: Lazy<Mutex<BTreeMap<String, usize>>> =
    Lazy::new(|| Mutex::new(BTreeMap::new()));
static TEST_NAME_CLASH_DETECTION: Lazy<Mutex<BTreeMap<String, bool>>> =
    Lazy::new(|| Mutex::new(BTreeMap::new()));
static INLINE_DUPLICATES: Lazy<Mutex<BTreeSet<String>>> = Lazy::new(|| Mutex::new(BTreeSet::new()));

thread_local! {
    static RECORDED_DUPLICATES: RefCell<Vec<BTreeMap<String, Snapshot>>> = RefCell::default()
}

// Writes to stderr and also to a warnings file (if INSTA_WARNINGS_FILE is set).
// The warnings file allows cargo-insta to display warnings after tests complete,
// since test runners like nextest suppress output from passing tests by default.
#[macro_export]
macro_rules! elog {
    ($($arg:tt)*) => {{
        use std::io::Write as _;
        let msg = format!($($arg)*);
        let _ = writeln!(std::io::stderr(), "{}", msg);
        $crate::env::memoize_warning(&msg);
    }};
}
#[cfg(feature = "glob")]
macro_rules! print_or_panic {
    ($fail_fast:expr, $($tokens:tt)*) => {{
        if (!$fail_fast) {
            eprintln!($($tokens)*);
            eprintln!();
        } else {
            panic!($($tokens)*);
        }
    }}
}

/// Special marker to use an automatic name.
///
/// This can be passed as a snapshot name in a macro to explicitly tell
/// insta to use the automatic name.  This is useful in ambiguous syntax
/// situations.
#[derive(Debug)]
pub struct AutoName;

pub struct InlineValue<'a>(pub &'a str);

/// The name of a snapshot, from which the path is derived.
type SnapshotName<'a> = Option<Cow<'a, str>>;

pub struct BinarySnapshotValue<'a> {
    pub name_and_extension: &'a str,
    pub content: Vec<u8>,
}

pub enum SnapshotValue<'a> {
    /// A text snapshot that gets stored along with the metadata in the same file.
    FileText {
        name: SnapshotName<'a>,

        /// The new generated value to compare against any previously approved content.
        content: &'a str,
    },

    /// An inline snapshot.
    InlineText {
        /// The reference content from the macro invocation that will be compared against.
        reference_content: &'a str,

        /// The new generated value to compare against any previously approved content.
        content: &'a str,
    },

    /// A text snapshot stored as a plain external file.
    ExternalText {
        /// The requested external file path.
        path: PathBuf,

        /// The new generated value to compare against any previously approved content.
        content: &'a str,
    },

    /// A binary snapshot that gets stored as a separate file next to the metadata file.
    Binary {
        name: SnapshotName<'a>,

        /// The new generated value to compare against any previously approved content.
        content: Vec<u8>,

        /// The extension of the separate file.
        extension: &'a str,
    },
}

impl<'a> SnapshotValue<'a> {
    #[doc(hidden)]
    pub fn external_file(path: impl AsRef<Path>, content: &'a str) -> Self {
        SnapshotValue::ExternalText {
            path: path.as_ref().to_path_buf(),
            content,
        }
    }
}

impl<'a> From<(AutoName, &'a str)> for SnapshotValue<'a> {
    fn from((_, content): (AutoName, &'a str)) -> Self {
        SnapshotValue::FileText {
            name: None,
            content,
        }
    }
}

impl<'a> From<(Option<String>, &'a str)> for SnapshotValue<'a> {
    fn from((name, content): (Option<String>, &'a str)) -> Self {
        SnapshotValue::FileText {
            name: name.map(Cow::Owned),
            content,
        }
    }
}

impl<'a> From<(String, &'a str)> for SnapshotValue<'a> {
    fn from((name, content): (String, &'a str)) -> Self {
        SnapshotValue::FileText {
            name: Some(Cow::Owned(name)),
            content,
        }
    }
}

impl<'a> From<(Option<&'a OsStr>, &'a str)> for SnapshotValue<'a> {
    fn from((name, content): (Option<&'a OsStr>, &'a str)) -> Self {
        SnapshotValue::FileText {
            name: name.map(OsStr::to_string_lossy),
            content,
        }
    }
}

impl<'a> From<(Option<&'a str>, &'a str)> for SnapshotValue<'a> {
    fn from((name, content): (Option<&'a str>, &'a str)) -> Self {
        SnapshotValue::FileText {
            name: name.map(Cow::Borrowed),
            content,
        }
    }
}

impl<'a> From<(&'a str, &'a str)> for SnapshotValue<'a> {
    fn from((name, content): (&'a str, &'a str)) -> Self {
        SnapshotValue::FileText {
            name: Some(Cow::Borrowed(name)),
            content,
        }
    }
}

impl<'a> From<(InlineValue<'a>, &'a str)> for SnapshotValue<'a> {
    fn from((InlineValue(reference_content), content): (InlineValue<'a>, &'a str)) -> Self {
        SnapshotValue::InlineText {
            reference_content,
            content,
        }
    }
}

impl<'a> From<BinarySnapshotValue<'a>> for SnapshotValue<'a> {
    fn from(
        BinarySnapshotValue {
            name_and_extension,
            content,
        }: BinarySnapshotValue<'a>,
    ) -> Self {
        let (name, extension) = name_and_extension.split_once('.').unwrap_or_else(|| {
            panic!("\"{name_and_extension}\" does not match the format \"name.extension\"",)
        });

        let name = if name.is_empty() {
            None
        } else {
            Some(Cow::Borrowed(name))
        };

        SnapshotValue::Binary {
            name,
            extension,
            content,
        }
    }
}

fn is_doctest(function_name: &str) -> bool {
    function_name.starts_with("rust_out::main::_doctest")
}

fn detect_snapshot_name(function_name: &str, module_path: &str) -> Result<String, &'static str> {
    // clean test name first
    let name = function_name.rsplit("::").next().unwrap();

    let (name, test_prefixed) = if let Some(stripped) = name.strip_prefix("test_") {
        (stripped, true)
    } else {
        (name, false)
    };

    // next check if we need to add a suffix
    let name = add_suffix_to_snapshot_name(Cow::Borrowed(name));
    let key = format!("{}::{}", module_path.replace("::", "__"), name);

    // because fn foo and fn test_foo end up with the same snapshot name we
    // make sure we detect this here and raise an error.
    let mut name_clash_detection = TEST_NAME_CLASH_DETECTION
        .lock()
        .unwrap_or_else(|x| x.into_inner());
    match name_clash_detection.get(&key) {
        None => {
            name_clash_detection.insert(key.clone(), test_prefixed);
        }
        Some(&was_test_prefixed) => {
            if was_test_prefixed != test_prefixed {
                panic!(
                    "Insta snapshot name clash detected between '{name}' \
                     and 'test_{name}' in '{module_path}'. Rename one function."
                );
            }
        }
    }

    // The rest of the code just deals with duplicates, which we in some
    // cases do not want to guard against.
    if allow_duplicates() {
        return Ok(name.to_string());
    }

    // if the snapshot name clashes we need to increment a counter.
    // we really do not care about poisoning here.
    let mut counters = TEST_NAME_COUNTERS.lock().unwrap_or_else(|x| x.into_inner());
    let test_idx = counters.get(&key).cloned().unwrap_or(0) + 1;
    let rv = if test_idx == 1 {
        name.to_string()
    } else {
        format!("{name}-{test_idx}")
    };
    counters.insert(key, test_idx);

    Ok(rv)
}

/// If there is a suffix on the settings, append it to the snapshot name.
fn add_suffix_to_snapshot_name(name: Cow<'_, str>) -> Cow<'_, str> {
    Settings::with(|settings| {
        settings
            .snapshot_suffix()
            .map(|suffix| Cow::Owned(format!("{name}@{suffix}")))
            .unwrap_or_else(|| name)
    })
}

fn get_snapshot_filename(
    module_path: &str,
    assertion_file: &str,
    snapshot_name: &str,
    cargo_workspace: &Path,
    is_doctest: bool,
) -> PathBuf {
    let root = Path::new(cargo_workspace);
    let base = Path::new(assertion_file);
    Settings::with(|settings| {
        root.join(base.parent().unwrap())
            .join(settings.snapshot_path())
            .join({
                use std::fmt::Write;
                let mut f = String::new();
                if settings.prepend_module_to_snapshot() {
                    if is_doctest {
                        write!(
                            &mut f,
                            "doctest_{}__",
                            base.file_name()
                                .unwrap()
                                .to_string_lossy()
                                .replace('.', "_")
                        )
                        .unwrap();
                    } else {
                        write!(&mut f, "{}__", module_path.replace("::", "__")).unwrap();
                    }
                }
                write!(
                    &mut f,
                    "{}.snap",
                    snapshot_name.replace(&['/', '\\'][..], "__")
                )
                .unwrap();
                f
            })
    })
}

fn snapshot_source(workspace: &Path, assertion_file: &str) -> String {
    let source_path = Path::new(assertion_file);
    // Canonicalize existing paths so workspace layouts and symlinks still
    // produce a workspace-relative source path.
    let canonicalized_base = workspace.canonicalize().ok();
    let canonicalized_path = source_path.canonicalize().ok();

    let relative = if let (Some(base), Some(path)) = (canonicalized_base, canonicalized_path) {
        path_relative_from(&path, &base).unwrap_or_else(|| source_path.to_path_buf())
    } else {
        path_relative_from(source_path, workspace).unwrap_or_else(|| source_path.to_path_buf())
    };
    path_to_storage(&relative)
}

fn normalize_path_lexically(path: &Path) -> PathBuf {
    use std::path::Component;

    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => match normalized.components().next_back() {
                Some(Component::Normal(_)) => {
                    normalized.pop();
                }
                Some(Component::ParentDir) | None if !path.has_root() => {
                    normalized.push(component.as_os_str());
                }
                _ => {}
            },
            _ => normalized.push(component.as_os_str()),
        }
    }
    normalized
}

fn external_snapshot_name(target_path: &Path, stored_path: &str) -> String {
    const MAX_BASENAME_BYTES: usize = 64;

    let basename = target_path
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or("external-file");
    let mut sanitized = String::new();
    for ch in basename.chars() {
        let ch = if ch.is_alphanumeric() || matches!(ch, '-' | '_' | '.') {
            ch
        } else {
            '_'
        };
        if sanitized.len() + ch.len_utf8() > MAX_BASENAME_BYTES {
            break;
        }
        sanitized.push(ch);
    }
    if sanitized.is_empty() {
        sanitized.push_str("external-file");
    }

    let hash = stable_external_snapshot_hash(stored_path);
    format!("{sanitized}@{hash:016x}")
}

/// Derives the pending snapshot name from its target.
///
/// The input deliberately excludes the assertion source and line so the hashed
/// name component survives edits around the assertion. Changing the input or
/// algorithm orphans every pending file users already have on disk.
fn stable_external_snapshot_hash(stored_path: &str) -> u64 {
    const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    let mut hash = FNV_OFFSET_BASIS;
    for byte in (stored_path.len() as u64)
        .to_le_bytes()
        .iter()
        .chain(stored_path.as_bytes())
    {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

#[derive(Debug)]
enum SnapshotDestination {
    ManagedFile {
        snapshot_file: PathBuf,
    },
    ExternalFile {
        target_path: PathBuf,
        pending_snapshot_file: PathBuf,
        stored_path: String,
    },
    Inline {
        pending_snapshots_file: PathBuf,
    },
}

impl SnapshotDestination {
    fn managed_file(snapshot_file: PathBuf) -> Self {
        Self::ManagedFile { snapshot_file }
    }

    fn inline(pending_snapshots_file: PathBuf) -> Self {
        Self::Inline {
            pending_snapshots_file,
        }
    }

    /// The file an accepted snapshot lands in, unless it is an inline one.
    fn target_file(&self) -> Option<&Path> {
        match self {
            SnapshotDestination::ManagedFile { snapshot_file } => Some(snapshot_file),
            SnapshotDestination::ExternalFile { target_path, .. } => Some(target_path),
            SnapshotDestination::Inline { .. } => None,
        }
    }

    /// The `.snap` file insta itself manages, if there is one.
    ///
    /// External targets are excluded on purpose. They are user owned files, so
    /// they take part in neither binary sidecar cleanup nor unreferenced
    /// snapshot tracking.
    fn snapshot_file(&self) -> Option<&Path> {
        match self {
            SnapshotDestination::ManagedFile { snapshot_file } => Some(snapshot_file),
            SnapshotDestination::ExternalFile { .. } | SnapshotDestination::Inline { .. } => None,
        }
    }

    /// The workspace relative target recorded in the snapshot metadata.
    fn stored_path(&self) -> Option<&str> {
        match self {
            SnapshotDestination::ExternalFile { stored_path, .. } => Some(stored_path),
            SnapshotDestination::ManagedFile { .. } | SnapshotDestination::Inline { .. } => None,
        }
    }

    fn is_inline(&self) -> bool {
        matches!(self, SnapshotDestination::Inline { .. })
    }

    fn text_kind(&self) -> TextSnapshotKind {
        if self.is_inline() {
            TextSnapshotKind::Inline
        } else {
            TextSnapshotKind::File
        }
    }
}

/// The context around a snapshot, such as the reference value, location, etc.
/// (but not including the generated value). Responsible for saving the
/// snapshot.
#[derive(Debug)]
struct SnapshotAssertionContext<'a> {
    tool_config: Arc<ToolConfig>,
    workspace: &'a Path,
    module_path: &'a str,
    snapshot_name: Option<Cow<'a, str>>,
    destination: SnapshotDestination,
    duplication_key: Option<String>,
    old_snapshot: Option<Snapshot>,
    assertion_file: &'a str,
    assertion_line: u32,
    is_doctest: bool,
    snapshot_kind: SnapshotKind,
}

impl<'a> SnapshotAssertionContext<'a> {
    fn prepare(
        new_snapshot_value: &SnapshotValue<'a>,
        workspace: &'a Path,
        function_name: &'a str,
        module_path: &'a str,
        assertion_file: &'a str,
        assertion_line: u32,
    ) -> Result<SnapshotAssertionContext<'a>, Box<dyn Error>> {
        let tool_config = get_tool_config(workspace);
        let snapshot_name;
        let mut duplication_key = None;
        let mut old_snapshot = None;
        let destination;
        let is_doctest = is_doctest(function_name);

        match new_snapshot_value {
            SnapshotValue::FileText { name, .. } | SnapshotValue::Binary { name, .. } => {
                let name = match &name {
                    Some(name) => add_suffix_to_snapshot_name(name.clone()),
                    None => {
                        if is_doctest {
                            panic!("Cannot determine reliable names for snapshot in doctests.  Please use explicit names instead.");
                        }
                        detect_snapshot_name(function_name, module_path)
                            .unwrap()
                            .into()
                    }
                };
                if allow_duplicates() {
                    duplication_key = Some(format!("named:{module_path}|{name}"));
                }
                let file = get_snapshot_filename(
                    module_path,
                    assertion_file,
                    &name,
                    workspace,
                    is_doctest,
                );
                if fs::metadata(&file).is_ok() {
                    match Snapshot::from_file(&file) {
                        Ok(snapshot) => {
                            old_snapshot = Some(snapshot);
                        }
                        Err(err) => {
                            // If we can't parse the snapshot (e.g., invalid YAML,
                            // merge conflicts, truncated file), log a warning and
                            // proceed. The test will generate a new pending snapshot.
                            elog!(
                                "{}: Failed to parse snapshot file; \
                                 a new snapshot will be generated: {}\n  Error: {}",
                                style("warning").yellow().bold(),
                                file.display(),
                                err
                            );
                        }
                    }
                }
                snapshot_name = Some(name);
                destination = SnapshotDestination::managed_file(file);
            }
            SnapshotValue::ExternalText { path, .. } => {
                let source_path = workspace.join(assertion_file);
                let target_path = if path.is_absolute() {
                    path.clone()
                } else {
                    source_path.parent().unwrap().join(path)
                };
                let target_path = normalize_path_lexically(&target_path);
                if target_path.to_str().is_none() {
                    return Err(format!(
                        "external snapshot path is not valid UTF-8: {}",
                        target_path.display()
                    )
                    .into());
                }

                let relative_path = path_relative_from(&target_path, workspace)
                    .unwrap_or_else(|| target_path.clone());
                let stored_path = path_to_storage(&relative_path);
                if allow_duplicates() {
                    duplication_key = Some(format!("external:{stored_path}"));
                }
                let name = external_snapshot_name(&target_path, &stored_path);
                let pending_snapshot_file = get_snapshot_filename(
                    module_path,
                    assertion_file,
                    &name,
                    workspace,
                    is_doctest,
                );

                old_snapshot = match fs::read_to_string(&target_path) {
                    Ok(contents) => Some(Snapshot::from_components(
                        module_path.replace("::", "__"),
                        Some(name.clone()),
                        MetaData::default(),
                        TextSnapshotContents::new(contents, TextSnapshotKind::File).into(),
                    )),
                    Err(error) if error.kind() == ErrorKind::NotFound => None,
                    Err(error) if error.kind() == ErrorKind::InvalidData => {
                        return Err(format!(
                            "external text snapshot is not valid UTF-8: {}",
                            target_path.display()
                        )
                        .into());
                    }
                    Err(error) => {
                        return Err(format!(
                            "failed to read external snapshot {}: {error}",
                            target_path.display()
                        )
                        .into());
                    }
                };
                snapshot_name = Some(Cow::Owned(name));
                destination = SnapshotDestination::ExternalFile {
                    target_path,
                    pending_snapshot_file,
                    stored_path,
                };
            }
            SnapshotValue::InlineText {
                reference_content: contents,
                ..
            } => {
                if allow_duplicates() {
                    duplication_key = Some(format!(
                        "inline:{function_name}|{assertion_file}|{assertion_line}"
                    ));
                } else {
                    prevent_inline_duplicate(function_name, assertion_file, assertion_line);
                }
                snapshot_name = detect_snapshot_name(function_name, module_path)
                    .ok()
                    .map(Cow::Owned);
                let mut pending_file = workspace.join(assertion_file);
                pending_file.set_file_name(format!(
                    ".{}.pending-snap",
                    pending_file
                        .file_name()
                        .expect("no filename")
                        .to_str()
                        .expect("non unicode filename")
                ));
                destination = SnapshotDestination::inline(pending_file);
                old_snapshot = Some(Snapshot::from_components(
                    module_path.replace("::", "__"),
                    None,
                    MetaData::default(),
                    SnapshotContents::Text(TextSnapshotContents::from_inline_literal(contents)),
                ));
            }
        };

        let snapshot_type = match new_snapshot_value {
            SnapshotValue::FileText { .. }
            | SnapshotValue::InlineText { .. }
            | SnapshotValue::ExternalText { .. } => SnapshotKind::Text,
            &SnapshotValue::Binary { extension, .. } => SnapshotKind::Binary {
                extension: extension.to_string(),
            },
        };

        Ok(SnapshotAssertionContext {
            tool_config,
            workspace,
            module_path,
            snapshot_name,
            destination,
            old_snapshot,
            assertion_file,
            assertion_line,
            duplication_key,
            is_doctest,
            snapshot_kind: snapshot_type,
        })
    }

    /// Given a path returns the local path within the workspace.
    pub fn localize_path(&self, p: &Path) -> Option<PathBuf> {
        let workspace = self.workspace.canonicalize().ok()?;
        let p = self.workspace.join(p).canonicalize().ok()?;
        p.strip_prefix(&workspace).ok().map(|x| x.to_path_buf())
    }

    /// Creates the new snapshot from input values.
    pub fn new_snapshot(&self, contents: SnapshotContents, expr: &str) -> Snapshot {
        assert_eq!(
            contents.is_binary(),
            matches!(self.snapshot_kind, SnapshotKind::Binary { .. })
        );

        let metadata = Settings::with(|settings| MetaData {
            source: Some(snapshot_source(self.workspace, self.assertion_file)),
            assertion_line: Some(self.assertion_line),
            description: settings.description().map(Into::into),
            expression: if settings.omit_expression() {
                None
            } else {
                Some(expr.to_string())
            },
            info: settings.info().map(ToOwned::to_owned),
            input_file: settings
                .input_file()
                .and_then(|x| self.localize_path(x))
                .map(|x| path_to_storage(&x)),
            path: self.destination.stored_path().map(ToOwned::to_owned),
            snapshot_kind: self.snapshot_kind.clone(),
        });

        Snapshot::from_components(
            self.module_path.replace("::", "__"),
            self.snapshot_name.as_ref().map(|x| x.to_string()),
            metadata,
            contents,
        )
    }

    /// Cleanup logic for passing snapshots.
    pub fn cleanup_passing(&self) -> Result<(), Box<dyn Error>> {
        // let's just make sure there are no more pending files lingering
        // around.
        match &self.destination {
            SnapshotDestination::ManagedFile { snapshot_file } => {
                let target_path = pending_snapshot_path(self.workspace, snapshot_file);
                let new_file = target_path.with_extension("snap.new");
                fs::remove_file(new_file).ok();
            }
            SnapshotDestination::ExternalFile {
                pending_snapshot_file,
                ..
            } => {
                let target_path = pending_snapshot_path(self.workspace, pending_snapshot_file);
                fs::remove_file(target_path.with_extension("snap.new")).ok();
            }
            SnapshotDestination::Inline {
                pending_snapshots_file,
            } => {
                // Add a null pending snapshot to a pending snapshot file if needed.
                let target_path = pending_snapshot_path(self.workspace, pending_snapshots_file);
                if fs::metadata(&target_path).is_ok() {
                    PendingInlineSnapshot::new(None, None, self.assertion_line)
                        .save(&target_path)?;
                }
            }
        }
        Ok(())
    }

    /// Removes any old .snap.new.* files that belonged to previous pending snapshots. This should
    /// only ever remove maximum one file because we do this every time before we create a new
    /// pending snapshot.
    pub fn cleanup_previous_pending_binary_snapshots(&self) -> Result<(), Box<dyn Error>> {
        if let Some(path) = self.destination.snapshot_file() {
            // Use pending directory if set
            let target_path = pending_snapshot_path(self.workspace, path);

            // The file name to compare against has to be valid utf-8 as it is generated by this crate
            // out of utf-8 strings.
            let file_name_prefix = format!(
                "{}.new.",
                target_path.file_name().unwrap().to_str().unwrap()
            );

            let read_dir = target_path.parent().unwrap().read_dir();

            match read_dir {
                Err(e) if e.kind() == ErrorKind::NotFound => return Ok(()),
                _ => (),
            }

            // We have to loop over where whole directory here because there is no filesystem API
            // for getting files by prefix.
            for entry in read_dir? {
                let entry = entry?;
                let entry_file_name = entry.file_name();

                // We'll just skip over files with non-utf-8 names. The assumption being that those
                // would not have been generated by this crate.
                if entry_file_name
                    .to_str()
                    .map(|f| f.starts_with(&file_name_prefix))
                    .unwrap_or(false)
                {
                    std::fs::remove_file(entry.path())?;
                }
            }
        }

        Ok(())
    }

    /// Writes the changes of the snapshot back.
    pub fn update_snapshot(
        &self,
        new_snapshot: Snapshot,
    ) -> Result<SnapshotUpdateBehavior, Box<dyn Error>> {
        // TODO: this seems to be making `unseen` be true when there is an
        // existing snapshot file; which seems wrong??
        let unseen = self
            .destination
            .target_file()
            .map_or(false, |x| fs::metadata(x).is_ok());
        let should_print = self.tool_config.output_behavior() != OutputBehavior::Nothing;
        let snapshot_update = snapshot_update_behavior(&self.tool_config, unseen);

        // If snapshot_update is `InPlace` and we have an inline snapshot, then
        // use `NewFile`, since we can't use `InPlace` for inline. `cargo-insta`
        // then accepts all snapshots at the end of the test.
        let snapshot_update =
            if snapshot_update == SnapshotUpdateBehavior::InPlace && self.destination.is_inline() {
                SnapshotUpdateBehavior::NewFile
            } else {
                snapshot_update
            };

        match snapshot_update {
            SnapshotUpdateBehavior::InPlace => match &self.destination {
                SnapshotDestination::ManagedFile { snapshot_file } => {
                    new_snapshot.save(snapshot_file)?;
                    if should_print {
                        elog!(
                            "{} {}",
                            style("updated snapshot").green(),
                            style(snapshot_file.display()).cyan().underlined(),
                        );
                    }
                }
                SnapshotDestination::ExternalFile { target_path, .. } => {
                    if let Some(parent) = target_path.parent() {
                        fs::create_dir_all(parent).map_err(|error| {
                            format!(
                                "failed to create external snapshot directory {}: {error}",
                                parent.display()
                            )
                        })?;
                    }
                    fs::write(
                        target_path,
                        new_snapshot
                            .as_text()
                            .expect("external text snapshot")
                            .to_string(),
                    )
                    .map_err(|error| {
                        format!(
                            "failed to write external snapshot {}: {error}",
                            target_path.display()
                        )
                    })?;
                    if should_print {
                        elog!(
                            "{} {}",
                            style("updated snapshot").green(),
                            style(target_path.display()).cyan().underlined(),
                        );
                    }
                }
                // `is_inline()` above rewrote `InPlace` to `NewFile`.
                SnapshotDestination::Inline { .. } => unreachable!(),
            },
            SnapshotUpdateBehavior::NewFile => match &self.destination {
                SnapshotDestination::ManagedFile { snapshot_file } => {
                    // File snapshot - use pending directory if set
                    let target_path = pending_snapshot_path(self.workspace, snapshot_file);
                    let new_path = new_snapshot.save_new(&target_path)?;
                    if should_print {
                        elog!(
                            "{} {}",
                            style("stored new snapshot").green(),
                            style(new_path.display()).cyan().underlined(),
                        );
                    }
                }
                SnapshotDestination::ExternalFile {
                    pending_snapshot_file,
                    ..
                } => {
                    let target_path = pending_snapshot_path(self.workspace, pending_snapshot_file);
                    let new_path = new_snapshot.save_new(&target_path)?;
                    if should_print {
                        elog!(
                            "{} {}",
                            style("stored new snapshot").green(),
                            style(new_path.display()).cyan().underlined(),
                        );
                    }
                }
                SnapshotDestination::Inline { .. } if self.is_doctest => {
                    if should_print {
                        elog!(
                            "{}",
                            style("warning: cannot update inline snapshots in doctests")
                                .red()
                                .bold(),
                        );
                    }
                }
                SnapshotDestination::Inline {
                    pending_snapshots_file,
                } => {
                    // Inline snapshot - use pending directory if set
                    let target_path = pending_snapshot_path(self.workspace, pending_snapshots_file);
                    PendingInlineSnapshot::new(
                        Some(new_snapshot),
                        self.old_snapshot.clone(),
                        self.assertion_line,
                    )
                    .save(&target_path)?;
                }
            },
            SnapshotUpdateBehavior::NoUpdate => {}
        }

        Ok(snapshot_update)
    }

    /// This prints the information about the snapshot
    fn print_snapshot_info(&self, new_snapshot: &Snapshot) {
        let mut printer =
            SnapshotPrinter::new(self.workspace, self.old_snapshot.as_ref(), new_snapshot);
        printer.set_line(Some(self.assertion_line));
        printer.set_snapshot_file(self.destination.target_file());
        printer.set_title(Some("Snapshot Summary"));
        printer.set_show_info(true);
        match self.tool_config.output_behavior() {
            OutputBehavior::Summary => {
                printer.print();
            }
            OutputBehavior::Diff => {
                printer.set_show_diff(true);
                printer.print();
            }
            _ => {}
        }
    }

    /// Finalizes the assertion when the snapshot comparison fails, potentially
    /// panicking to fail the test
    fn finalize(&self, update_result: SnapshotUpdateBehavior) {
        // if we are in glob mode, we want to adjust the finalization
        // so that we do not show the hints immediately.
        let fail_fast = {
            #[cfg(feature = "glob")]
            {
                if let Some(top) = crate::glob::GLOB_STACK.lock().unwrap().last() {
                    top.fail_fast
                } else {
                    true
                }
            }
            #[cfg(not(feature = "glob"))]
            {
                true
            }
        };

        if fail_fast
            && update_result == SnapshotUpdateBehavior::NewFile
            && self.tool_config.output_behavior() != OutputBehavior::Nothing
            && !self.is_doctest
        {
            // `INSTA_UPDATE=always` only bypasses review for file snapshots;
            // inline snapshots always go through a pending file.
            let hint = if self.destination.is_inline() {
                "To update snapshots run `cargo insta review`"
            } else {
                "To update snapshots run `cargo insta review` or set `INSTA_UPDATE=always`"
            };
            println!("{hint}", hint = style(hint).dim());
        }

        if update_result != SnapshotUpdateBehavior::InPlace && !self.tool_config.force_pass() {
            if fail_fast && self.tool_config.output_behavior() != OutputBehavior::Nothing {
                let msg = if env::var("INSTA_CARGO_INSTA") == Ok("1".to_string()) {
                    "Stopped on the first failure."
                } else {
                    "Stopped on the first failure. Run `cargo insta test` to run all snapshots."
                };
                println!("{hint}", hint = style(msg).dim(),);
            }

            // if we are in glob mode, count the failures and print the
            // errors instead of panicking.  The glob will then panic at
            // the end.
            #[cfg(feature = "glob")]
            {
                let mut stack = crate::glob::GLOB_STACK.lock().unwrap();
                if let Some(glob_collector) = stack.last_mut() {
                    glob_collector.failed += 1;
                    if update_result == SnapshotUpdateBehavior::NewFile
                        && self.tool_config.output_behavior() != OutputBehavior::Nothing
                        && !self.destination.is_inline()
                    {
                        glob_collector.show_insta_hint = true;
                    }

                    print_or_panic!(
                        fail_fast,
                        "snapshot assertion from glob for '{}' failed in line {}",
                        self.snapshot_name.as_deref().unwrap_or("unnamed snapshot"),
                        self.assertion_line
                    );
                    return;
                }
            }

            panic!(
                "snapshot assertion for '{}' failed in line {}",
                self.snapshot_name.as_deref().unwrap_or("unnamed snapshot"),
                self.assertion_line
            );
        }
    }
}

/// Computes a relative path from `base` to `path`, returning a path with `../` components
/// if necessary.
///
/// This function is vendored from the old Rust standard library implementation
/// (pre-1.0, removed in RFC 474) and is distributed under the same terms as the
/// Rust project (MIT/Apache-2.0 dual license).
///
/// Unlike `Path::strip_prefix`, this function can handle cases where `path` is not
/// a descendant of `base`, making it suitable for finding relative paths between
/// arbitrary directories (e.g., between sibling directories in a workspace).
fn path_relative_from(path: &Path, base: &Path) -> Option<PathBuf> {
    use std::path::Component;

    if path.is_absolute() != base.is_absolute() {
        if path.is_absolute() {
            Some(PathBuf::from(path))
        } else {
            None
        }
    } else {
        let mut ita = path.components();
        let mut itb = base.components();
        let mut comps: Vec<Component> = vec![];
        loop {
            match (ita.next(), itb.next()) {
                (None, None) => break,
                (Some(a), None) => {
                    comps.push(a);
                    comps.extend(ita.by_ref());
                    break;
                }
                (None, _) => comps.push(Component::ParentDir),
                (Some(a), Some(b)) if comps.is_empty() && a == b => {}
                (Some(a), Some(_b)) => {
                    comps.push(Component::ParentDir);
                    for _ in itb {
                        comps.push(Component::ParentDir);
                    }
                    comps.push(a);
                    comps.extend(ita.by_ref());
                    break;
                }
            }
        }
        Some(comps.iter().map(|c| c.as_os_str()).collect())
    }
}

fn prevent_inline_duplicate(function_name: &str, assertion_file: &str, assertion_line: u32) {
    let key = format!("{function_name}|{assertion_file}|{assertion_line}");
    let mut set = INLINE_DUPLICATES.lock().unwrap();
    if set.contains(&key) {
        // drop the lock so we don't poison it
        drop(set);
        panic!(
            "Insta does not allow inline snapshot assertions in loops. \
            Wrap your assertions in allow_duplicates! to change this."
        );
    }
    set.insert(key);
}

fn record_snapshot_duplicate(
    results: &mut BTreeMap<String, Snapshot>,
    snapshot: &Snapshot,
    ctx: &SnapshotAssertionContext,
) {
    let key = ctx.duplication_key.as_deref().unwrap();
    if let Some(prev_snapshot) = results.get(key) {
        if prev_snapshot.contents() != snapshot.contents() {
            println!("Snapshots in allow-duplicates block do not match.");
            let mut printer = SnapshotPrinter::new(ctx.workspace, Some(prev_snapshot), snapshot);
            printer.set_line(Some(ctx.assertion_line));
            printer.set_snapshot_file(ctx.destination.target_file());
            printer.set_title(Some("Differences in Block"));
            printer.set_snapshot_hints("previous assertion", "current assertion");
            if ctx.tool_config.output_behavior() == OutputBehavior::Diff {
                printer.set_show_diff(true);
            }
            printer.print();
            panic!(
                "snapshot assertion for '{}' failed in line {}. Result \
                    does not match previous snapshot in allow-duplicates block.",
                ctx.snapshot_name.as_deref().unwrap_or("unnamed snapshot"),
                ctx.assertion_line
            );
        }
    } else {
        results.insert(key.to_string(), snapshot.clone());
    }
}

/// Do we allow recording of duplicates?
fn allow_duplicates() -> bool {
    RECORDED_DUPLICATES.with(|x| !x.borrow().is_empty())
}

/// Helper function to support perfect duplicate detection.
pub fn with_allow_duplicates<R, F>(f: F) -> R
where
    F: FnOnce() -> R,
{
    RECORDED_DUPLICATES.with(|x| x.borrow_mut().push(BTreeMap::new()));
    let rv = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    RECORDED_DUPLICATES.with(|x| x.borrow_mut().pop().unwrap());
    match rv {
        Ok(rv) => rv,
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

/// This function is invoked from the macros to run the main assertion logic.
///
/// This will create the assertion context, run the main logic to assert
/// on snapshots and write changes to the pending snapshot files.  It will
/// also print the necessary bits of information to the output and fail the
/// assertion with a panic if needed.
#[allow(clippy::too_many_arguments)]
pub fn assert_snapshot(
    snapshot_value: SnapshotValue<'_>,
    workspace: &Path,
    function_name: &str,
    module_path: &str,
    assertion_file: &str,
    assertion_line: u32,
    expr: &str,
) -> Result<(), Box<dyn Error>> {
    let mut ctx = SnapshotAssertionContext::prepare(
        &snapshot_value,
        workspace,
        function_name,
        module_path,
        assertion_file,
        assertion_line,
    )?;

    ctx.cleanup_previous_pending_binary_snapshots()?;

    let content = match snapshot_value {
        SnapshotValue::FileText { content, .. }
        | SnapshotValue::InlineText { content, .. }
        | SnapshotValue::ExternalText { content, .. } => {
            // strip ANSI escape codes if enabled
            #[cfg(feature = "filters")]
            let content = Settings::with(|settings| {
                if settings.strip_ansi_escape_codes() {
                    crate::filters::strip_ansi_escape_codes(content)
                } else {
                    std::borrow::Cow::Borrowed(content)
                }
            });

            // apply filters if they are available
            #[cfg(feature = "filters")]
            let content = Settings::with(|settings| settings.filters().apply_to(&content));

            let kind = ctx.destination.text_kind();

            TextSnapshotContents::new(content.into(), kind).into()
        }
        SnapshotValue::Binary {
            content, extension, ..
        } => {
            assert!(
                extension != "new",
                "'.new' is not allowed as a file extension"
            );
            assert!(
                !extension.starts_with("new."),
                "file extensions starting with 'new.' are not allowed",
            );

            SnapshotContents::Binary(Some(Rc::new(content)))
        }
    };

    let new_snapshot = ctx.new_snapshot(content, expr);

    if matches!(ctx.destination, SnapshotDestination::ExternalFile { .. }) {
        if let Some(old_snapshot) = ctx.old_snapshot.as_mut() {
            old_snapshot.metadata = new_snapshot.metadata().clone();
        }
    }

    // memoize the snapshot file if requested, as part of potentially removing unreferenced snapshots
    if let Some(snapshot_file) = ctx.destination.snapshot_file() {
        memoize_snapshot_file(snapshot_file);
    }

    // If we allow assertion with duplicates, we record the duplicate now.  This will
    // in itself fail the assertion if the previous visit of the same assertion macro
    // did not yield the same result.
    RECORDED_DUPLICATES.with(|x| {
        if let Some(results) = x.borrow_mut().last_mut() {
            record_snapshot_duplicate(results, &new_snapshot, &ctx);
        }
    });

    let pass = Settings::with(|settings| {
        ctx.old_snapshot
            .as_ref()
            .map(|x| {
                if ctx.tool_config.require_full_match() {
                    settings.comparator().matches_fully(x, &new_snapshot)
                } else {
                    settings.comparator().matches(x, &new_snapshot)
                }
            })
            .unwrap_or(false)
    });

    if pass {
        ctx.cleanup_passing()?;

        if matches!(
            ctx.tool_config.snapshot_update(),
            crate::env::SnapshotUpdate::Force
        ) {
            ctx.update_snapshot(new_snapshot)?;
        }
    // otherwise print information and update snapshots.
    } else {
        ctx.print_snapshot_info(&new_snapshot);
        let update_result = ctx.update_snapshot(new_snapshot)?;
        ctx.finalize(update_result);
    }

    Ok(())
}

#[allow(rustdoc::private_doc_tests)]
/// Test snapshots in doctests.
///
/// ```
/// // this is only working on newer rust versions
/// extern crate rustc_version;
/// use rustc_version::{Version, version};
/// if version().unwrap() > Version::parse("1.72.0").unwrap() {
///     insta::assert_debug_snapshot!("named", vec![1, 2, 3, 4, 5]);
/// }
/// ```
///
/// ```should_panic
/// insta::assert_debug_snapshot!(vec![1, 2, 3, 4, 5]);
/// ```
///
/// ```
/// let some_string = "Coucou je suis un joli bug";
/// insta::assert_snapshot!(some_string, @"Coucou je suis un joli bug");
/// ```
///
/// ```
/// let some_string = "Coucou je suis un joli bug";
/// insta::assert_snapshot!(some_string, @"Coucou je suis un joli bug");
/// ```
const _DOCTEST1: bool = false;

#[cfg(test)]
mod external_snapshot_tests {
    use super::*;

    #[test]
    fn external_snapshot_name_is_stable_and_sanitized() {
        let name =
            external_snapshot_name(Path::new("generated/output.txt"), "generated/output.txt");
        assert_eq!(name, "output.txt@c37e249283e88658");

        let name = external_snapshot_name(
            Path::new("generated/file name?.txt"),
            "generated/file name?.txt",
        );
        assert!(name.starts_with("file_name_.txt@"), "{name}");
    }

    #[test]
    fn external_snapshot_names_distinguish_targets() {
        let name = |stored: &str| external_snapshot_name(Path::new(stored), stored);
        assert_ne!(name("generated/a.txt"), name("generated/b.txt"));
    }
}
