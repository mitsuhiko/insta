//! <div align="center">
//!  <img src="https://github.com/mitsuhiko/insta/blob/master/assets/logo.png?raw=true" width="250" height="250">
//!  <p><strong>insta: a snapshot testing library for Rust</strong></p>
//! </div>
//!
//! # How it Operates
//!
//! This crate exports multiple macros for snapshot testing:
//!
//! - `assert_snapshot_matches!` for comparing basic string snapshots.
//! - `assert_debug_snapshot_matches!` for comparing `Debug` outputs of values.
//! - `assert_serialized_snapshot_matches!` for comparing YAML serialized
//!   output of types implementing `serde::Serialize`.
//! - `assert_ron_snapshot_matches!` for comparing RON serialized output of
//!   types implementing `serde::Serialize`.
//! - `assert_json_snapshot_matches!` for comparing JSON serialized output of
//!   types implementing `serde::Serialize`.
//!
//! Snapshots are stored in the `snapshots` folder right next to the test file
//! where this is used.  The name of the file is `<module>__<name>.snap` where
//! the `name` of the snapshot has to be provided to the assertion macro.
//!
//! For macros that work with `serde::Serialize` this crate also permits
//! redacting of partial values.  See [redactions](#redactions) for more
//! information.
//!
//! <img src="https://github.com/mitsuhiko/insta/blob/master/assets/insta.gif?raw=true" alt="">
//!
//! # Example
//!
//! Install `insta` and `cargo-insta`:
//!
//! ```ignore
//! $ cargo add --dev insta
//! $ cargo install cargo-insta
//! ```
//!
//! ```rust,ignore
//! use insta::assert_debug_snapshot_matches;
//!
//! #[test]
//! fn test_snapshots() {
//!     let value = vec![1, 2, 3];
//!     assert_debug_snapshot_matches!("snapshot_name", value);
//! }
//! ```
//!
//! The recommended flow is to run the tests once, have them fail and check
//! if the result is okay.  By default the new snapshots are stored next
//! to the old ones with the extra `.new` extension.  Once you are satisifed
//! move the new files over.  You can also use `cargo insta review` which
//! will let you interactively review them:
//!
//! ```ignore
//! $ cargo test
//! $ cargo insta review
//! ```
//!
//! For more information on updating see [Snapshot Updating].
//!
//! [Snapshot Updating]: #snapshot-updating
//!
//! # Snapshot files
//!
//! The committed snapshot files will have a header with some meta information
//! that can make debugging easier and the snapshot:
//!
//! ```ignore
//! ---
//! created: "2019-01-21T22:03:13.792906+00:00"
//! creator: insta@0.3.0
//! expression: "&User{id: Uuid::new_v4(), username: \"john_doe\".to_string(),}"
//! source: tests/test_redaction.rs
//! ---
//! [
//!     1,
//!     2,
//!     3
//! ]
//! ```
//!
//! # Snapshot Updating
//!
//! During test runs snapshots will be updated according to the `INSTA_UPDATE`
//! environment variable.  The default is `auto` which will write all new
//! snapshots into `.snap.new` files if no CI is detected.
//!
//! `INSTA_UPDATE` modes:
//!
//! - `auto`: the default. `no` for CI environments or `new` otherwise
//! - `always`: overwrites old snapshot files with new ones unasked
//! - `new`: write new snapshots into `.snap.new` files.
//! - `no`: does not update snapshot files at all (just runs tests)
//!
//! When `new` is used as mode the `cargo-insta` command can be used to review
//! the snapshots conveniently:
//!
//! ```ignore
//! $ cargo install cargo-insta
//! $ cargo test
//! $ cargo insta review
//! ```
//!
//! "enter" or "a" accepts a new snapshot, "escape" or "r" rejects,
//! "space" or "s" skips the snapshot for now.
//!
//! For more information invoke `cargo insta --help`.
//!
//! # Test Assertions
//!
//! By default the tests will fail when the snapshot assertion fails.  However
//! if a test produces more than one snapshot it can be useful to force a test
//! to pass so that all new snapshots are created in one go.
//!
//! This can be enabled by setting `INSTA_FORCE_PASS` to `1`:
//!
//! ```ignore
//! $ INSTA_FORCE_PASS=1 cargo test --no-fail-fast
//! ```
//!
//! # Redactions
//!
//! For all snapshots created based on `serde::Serialize` output `insta`
//! supports redactions.  This permits replacing values with hardcoded other
//! values to make snapshots stable when otherwise random or otherwise changing
//! values are involved.
//!
//! Redactions can be defined as the third argument to those macros with
//! the syntax `{ selector => replacement_value }`.
//!
//! The following selectors exist:
//!
//! - `.key`: selects the given key
//! - `["key"]`: alternative syntax for keys
//! - `[index]`: selects the given index in an array
//! - `[]`: selects all items on an array
//! - `[:end]`: selects all items up to `end` (excluding, supports negative indexing)
//! - `[start:]`: selects all items starting with `start`
//! - `[start:end]`: selects all items from `start` to `end` (end excluding,
//!   supports negative indexing).
//! - `.*`: selects all keys on that depth
//!
//! Example usage:
//!
//! ```rust,ignore
//! #[derive(Serialize)]
//! pub struct User {
//!     id: Uuid,
//!     username: String,
//! }
//!
//! assert_serialized_snapshot_matches!("user", &User {
//!     id: Uuid::new_v4(),
//!     username: "john_doe".to_string(),
//! }, {
//!     ".id" => "[uuid]"
//! });
//! ```
#[macro_use]
mod macros;
mod content;
mod redaction;
mod runtime;
mod serialization;
mod snapshot;

#[cfg(test)]
mod test;

pub use crate::snapshot::Snapshot;

// exported for cargo-insta only
#[doc(hidden)]
pub use crate::{runtime::print_snapshot_diff, snapshot::PendingInlineSnapshot};

// these are here to make the macros work
#[doc(hidden)]
pub mod _macro_support {
    pub use crate::content::Content;
    pub use crate::redaction::Selector;
    pub use crate::runtime::{assert_snapshot, ReferenceValue};
    pub use crate::serialization::{
        serialize_value, serialize_value_redacted, SerializationFormat,
    };
}
