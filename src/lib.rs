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
//! ```no_run
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
//! to the old ones with the extra `.new` extension.  Once you are satisifed
//! move the new files over.  To simplify this workflow you can use
//! `cargo insta review` which will let you interactively review them:
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
//! - `assert_snapshot!` for comparing basic string snapshots.
//! - `assert_debug_snapshot!` for comparing `Debug` outputs of values.
//! - `assert_display_snapshot!` for comparing `Display` outputs of values.
//! - `assert_csv_snapshot!` for comparing CSV serialized output of
//!   types implementing `serde::Serialize`. (requires the `csv` feature)
//! - `assert_toml_snapshot!` for comparing TOML serialized output of
//!   types implementing `serde::Serialize`. (requires the `toml` feature)
//! - `assert_yaml_snapshot!` for comparing YAML serialized
//!   output of types implementing `serde::Serialize`.
//! - `assert_ron_snapshot!` for comparing RON serialized output of
//!   types implementing `serde::Serialize`. (requires the `ron` feature)
//! - `assert_json_snapshot!` for comparing JSON serialized output of
//!   types implementing `serde::Serialize`.
//!
//! For macros that work with `serde::Serialize` this crate also permits
//! redacting of partial values.  See [redactions in the documentation](https://insta.rs/docs/redactions/)
//! for more information.
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
//! ```no_run
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
//! * `csv`: enables CSV support ([`assert_csv_snapshot!`])
//! * `ron`: enables RON support ([`assert_ron_snapshot!`])
//! * `toml`: enables TOML support ([`assert_toml_snapshot!`])
//! * `redactions`: enables support for redactions
//! * `glob`: enables support for globbing ([`glob!`])
//! * `colors`: enables color output (enabled by default)
//!
//! # Settings
//!
//! There are some settings that can be changed on a per-thread (and thus
//! per-test) basis.  For more information see [Settings].
#[macro_use]
mod macros;
mod content;
mod env;
mod output;
mod runtime;
mod serialization;
mod settings;
mod snapshot;
mod utils;

#[cfg(feature = "redactions")]
mod redaction;

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
    pub use crate::runtime::AutoName;
    pub use crate::snapshot::{MetaData, SnapshotContents};
    #[cfg(feature = "redactions")]
    pub use crate::{
        redaction::{ContentPath, Redaction},
        settings::Redactions,
    };
}

// exported for cargo-insta only
#[doc(hidden)]
pub mod _cargo_insta_support {
    pub use crate::{
        output::print_snapshot_diff, snapshot::PendingInlineSnapshot, snapshot::SnapshotContents,
    };
}

// useful for redactions
#[cfg(feature = "redactions")]
pub use crate::redaction::{dynamic_redaction, sorted_redaction};

// these are here to make the macros work
#[doc(hidden)]
pub mod _macro_support {
    pub use crate::content::Content;
    pub use crate::env::get_cargo_workspace;
    pub use crate::runtime::{assert_snapshot, AutoName, ReferenceValue};
    pub use crate::serialization::{serialize_value, SerializationFormat, SnapshotLocation};

    #[cfg(feature = "glob")]
    pub use crate::glob::glob_exec;

    #[cfg(feature = "redactions")]
    pub use crate::{
        redaction::Redaction, redaction::Selector, serialization::serialize_value_redacted,
    };
}
