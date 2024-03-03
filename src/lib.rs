//! <div align="center">
//!  <img src="https://github.com/mitsuhiko/insta/blob/master/assets/logo.png?raw=true" width="250" height="250">
//!  <p><strong>insta: a snapshot testing library for Rust</strong></p>
//! </div>
//!
//! # What are snapshot tests
//!
//! Snapshots tests (also sometimes called approval tests) are tests that
//! assert values against a reference value (the snapshot).  This is similar
//! to how `assert_eq!` lets you compare a value against a reference value but
//! unlike simple string assertions, snapshot tests let you test against complex
//! values and come with comprehensive tools to review changes.
//!
//! Snapshot tests are particularly useful if your reference values are very
//! large or change often.
//!
//! # What it looks like:
//!
//! ```no_run
//! #[test]
//! fn test_hello_world() {
//!     insta::assert_debug_snapshot!(vec![1, 2, 3]);
//! }
//! ```
//!
//! Where are the snapshots stored?  Right next to your test in a folder
//! called `snapshots` as individual [`.snap` files](https://insta.rs/docs/snapshot-files/).
//!
//! Got curious?
//!
//! * [Read the introduction](https://insta.rs/docs/quickstart/)
//! * [Read the main documentation](https://insta.rs/docs/) which does not just
//!   cover the API of the crate but also many of the details of how it works.
//! * There is a screencast that shows the entire workflow: [watch the insta
//! introduction screencast](https://www.youtube.com/watch?v=rCHrMqE4JOY&feature=youtu.be).
//!
//! # Writing Tests
//!
//! ```
//! use insta::assert_debug_snapshot;
//!
//! #[test]
//! fn test_snapshots() {
//!     assert_debug_snapshot!(vec![1, 2, 3]);
//! }
//! ```
//!
//! The recommended flow is to run the tests once, have them fail and check
//! if the result is okay.  By default the new snapshots are stored next
//! to the old ones with the extra `.new` extension.  Once you are satisfied
//! move the new files over.  To simplify this workflow you can use
//! `cargo insta review` (requires
//! [`cargo-insta`](https://crates.io/crates/cargo-insta)) which will let you
//! interactively review them:
//!
//! ```text
//! $ cargo test
//! $ cargo insta review
//! ```
//!
//! # Assertion Macros
//!
//! This crate exports multiple macros for snapshot testing:
//!
//! - [`assert_snapshot!`] for comparing basic string snapshots.
//! - [`assert_debug_snapshot!`] for comparing [`Debug`] outputs of values.
//! - [`assert_display_snapshot!`] for comparing [`Display`](std::fmt::Display) outputs of values.
//!
//! The following macros require the use of serde's [`Serialize`](serde::Serialize):
//!
#![cfg_attr(
    feature = "csv",
    doc = "- [`assert_csv_snapshot!`] for comparing CSV serialized output. (requires the `csv` feature)"
)]
#![cfg_attr(
    feature = "toml",
    doc = "- [`assert_toml_snapshot!`] for comparing TOML serialized output. (requires the `toml` feature)"
)]
#![cfg_attr(
    feature = "yaml",
    doc = "- [`assert_yaml_snapshot!`] for comparing YAML serialized output. (requires the `yaml` feature)"
)]
#![cfg_attr(
    feature = "ron",
    doc = "- [`assert_ron_snapshot!`] for comparing RON serialized output. (requires the `ron` feature)"
)]
#![cfg_attr(
    feature = "json",
    doc = "- [`assert_json_snapshot!`] for comparing JSON serialized output. (requires the `json` feature)"
)]
#![cfg_attr(
    feature = "json",
    doc = "- [`assert_compact_json_snapshot!`] for comparing JSON serialized output while preferring single-line formatting. (requires the `json` feature)"
)]
//!
//! For macros that work with [`serde`] this crate also permits redacting of
//! partial values.  See [redactions in the
//! documentation](https://insta.rs/docs/redactions/) for more information.
//!
//! # Snapshot updating
//!
//! During test runs snapshots will be updated according to the `INSTA_UPDATE`
//! environment variable.  The default is `auto` which will write all new
//! snapshots into `.snap.new` files if no CI is detected so that
//! [`cargo-insta`](https://crates.io/crates/cargo-insta)
//! can pick them up.  Normally you don't have to change this variable.
//!
//! `INSTA_UPDATE` modes:
//!
//! - `auto`: the default. `no` for CI environments or `new` otherwise
//! - `always`: overwrites old snapshot files with new ones unasked
//! - `unseen`: behaves like `always` for new snapshots and `new` for others
//! - `new`: write new snapshots into `.snap.new` files
//! - `no`: does not update snapshot files at all (just runs tests)
//!
//! When `new` or `auto` is used as mode the [`cargo-insta`](https://crates.io/crates/cargo-insta)
//! command can be used to review the snapshots conveniently:
//!
//! ```text
//! $ cargo insta review
//! ```
//!
//! "enter" or "a" accepts a new snapshot, "escape" or "r" rejects,
//! "space" or "s" skips the snapshot for now.
//!
//! For more information [read the cargo insta docs](https://insta.rs/docs/cli/).
//!
//! # Inline Snapshots
//!
//! Additionally snapshots can also be stored inline.  In that case the format
//! for the snapshot macros is `assert_snapshot!(reference_value, @"snapshot")`.
//! The leading at sign (`@`) indicates that the following string is the
//! reference value.  `cargo-insta` will then update that string with the new
//! value on review.
//!
//! Example:
//!
#![cfg_attr(feature = "yaml", doc = " ```no_run")]
#![cfg_attr(not(feature = "yaml"), doc = " ```ignore")]
//! # use insta::*; use serde::Serialize;
//! #[derive(Serialize)]
//! pub struct User {
//!     username: String,
//! }
//!
//! assert_yaml_snapshot!(User {
//!     username: "john_doe".to_string(),
//! }, @"");
//! ```
//!
//! Like with normal snapshots after the initial test failure you can run
//! `cargo insta review` to accept the change.  The file will then be updated
//! automatically.
//!
//! # Features
//!
//! The following features exist:
//!
//! * `csv`: enables CSV support (via serde)
//! * `json`: enables JSON support (via serde)
//! * `ron`: enables RON support (via serde)
//! * `toml`: enables TOML support (via serde)
//! * `yaml`: enables YAML support (via serde)
//! * `redactions`: enables support for redactions
//! * `filters`: enables support for filters
//! * `glob`: enables support for globbing ([`glob!`])
//! * `colors`: enables color output (enabled by default)
//!
//! For legacy reasons the `json` and `yaml` features are enabled by default
//! in limited capacity.  You will receive a deprecation warning if you are
//! not opting into them but for now the macros will continue to function.
//!
//! Enabling any of the serde based formats enables the hidden `serde` feature
//! which gates some serde specific APIs such as [`Settings::set_info`].
//!
//! # Dependencies
//!
//! `insta` tries to be light in dependencies but this is tricky to accomplish
//! given what it tries to do.  By default it currently depends on `serde` for
//! the [`assert_toml_snapshot!`] and [`assert_yaml_snapshot!`] macros.  In
//! the future this default dependencies will be removed.  To already benefit
//! from this optimization you can disable the default features and manually
//! opt into what you want.
//!
//! # Settings
//!
//! There are some settings that can be changed on a per-thread (and thus
//! per-test) basis.  For more information see [Settings].
//!
//! Additionally Insta will load a YAML config file with settings that change
//! the behavior of insta between runs.  It's loaded from any of the following
//! locations: `.config/insta.yaml`, `insta.yaml` and `.insta.yaml` from the
//! workspace root.  The following config options exist:
//!
//! ```yaml
//! behavior:
//!   # also set by INSTA_FORCE_UPDATE
//!   force_update: true/false
//!   # also set by INSTA_REQUIRE_FULL_MATCH
//!   require_full_match: true/false
//!   # also set by INSTA_FORCE_PASS
//!   force_pass: true/false
//!   # also set by INSTA_OUTPUT
//!   output: "diff" | "summary" | "minimal" | "none"
//!   # also set by INSTA_UPDATE
//!   update: "auto" | "always" | "new" | "unseen" | "no"
//!   # also set by INSTA_GLOB_FAIL_FAST
//!   glob_fail_fast: true/false
//!
//! # these are used by cargo insta test
//! test:
//!   # also set by INSTA_TEST_RUNNER
//!   runner: "auto" | "cargo-test" | "nextest"
//!   # automatically assume --review was passed to cargo insta test
//!   auto_review: true/false
//!   # automatically assume --accept-unseen was passed to cargo insta test
//!   auto_accept_unseen: true/false
//!
//! # these are used by cargo insta review
//! review:
//!   # also look for snapshots in ignored folders
//!   include_ignored: true / false
//!   # also look for snapshots in hidden folders
//!   include_hidden: true / false
//!   # show a warning if undiscovered (ignored or hidden) snapshots are found.
//!   # defaults to true but creates a performance hit.
//!   warn_undiscovered: true / false
//! ```
//!
//! # Optional: Faster Runs
//!
//! Insta benefits from being compiled in release mode, even as dev dependency.  It
//! will compile slightly slower once, but use less memory, have faster diffs and
//! just generally be more fun to use.  To achieve that, opt `insta` and `similar`
//! (the diffing library) into higher optimization in your `Cargo.toml`:
//!
//! ```yaml
//! [profile.dev.package.insta]
//! opt-level = 3
//!
//! [profile.dev.package.similar]
//! opt-level = 3
//! ```
#![cfg_attr(docsrs, feature(doc_cfg))]

#[macro_use]
mod macros;
mod content;
mod env;
mod output;
mod runtime;
#[cfg(feature = "serde")]
mod serialization;
mod settings;
mod snapshot;
mod utils;

#[cfg(feature = "redactions")]
mod redaction;

#[cfg(feature = "filters")]
mod filters;

#[cfg(feature = "glob")]
mod glob;

#[cfg(test)]
mod test;

pub use crate::settings::Settings;
pub use crate::snapshot::{MetaData, Snapshot};

/// Exposes some library internals.
///
/// You're unlikely to want to work with these objects but they
/// are exposed for documentation primarily.
pub mod internals {
    pub use crate::content::Content;
    #[cfg(feature = "filters")]
    pub use crate::filters::Filters;
    pub use crate::runtime::AutoName;
    pub use crate::settings::SettingsBindDropGuard;
    pub use crate::snapshot::{MetaData, SnapshotContents};
    #[cfg(feature = "redactions")]
    pub use crate::{
        redaction::{ContentPath, Redaction},
        settings::Redactions,
    };
}

// exported for cargo-insta only
#[doc(hidden)]
#[cfg(feature = "_cargo_insta_internal")]
pub mod _cargo_insta_support {
    pub use crate::{
        content::Error as ContentError,
        env::{
            Error as ToolConfigError, OutputBehavior, SnapshotUpdate, TestRunner, ToolConfig,
            UnreferencedSnapshots,
        },
        output::SnapshotPrinter,
        snapshot::PendingInlineSnapshot,
        snapshot::SnapshotContents,
        utils::is_ci,
    };
}

// useful for redactions
#[cfg(feature = "redactions")]
pub use crate::redaction::{dynamic_redaction, rounded_redaction, sorted_redaction};

// these are here to make the macros work
#[doc(hidden)]
pub mod _macro_support {
    pub use crate::content::Content;
    pub use crate::env::get_cargo_workspace;
    pub use crate::runtime::{assert_snapshot, with_allow_duplicates, AutoName, ReferenceValue};

    #[cfg(feature = "serde")]
    pub use crate::serialization::{serialize_value, SerializationFormat, SnapshotLocation};

    #[cfg(feature = "glob")]
    pub use crate::glob::glob_exec;

    #[cfg(feature = "redactions")]
    pub use crate::{
        redaction::Redaction, redaction::Selector, serialization::serialize_value_redacted,
    };
}
