//! <div align="center">
//!  <img src="https://github.com/mitsuhiko/insta/blob/master/assets/logo.png?raw=true" width="250" height="250">
//!  <p><strong>insta: a snapshot testing library for Rust</strong></p>
//! </div>
//!
//! # How it Operates
//!
//! This crate exports two basic macros for snapshot testing:
//! `assert_snapshot_matches!` for comparing basic string snapshots and
//! `assert_debug_snapshot_matches!` for snapshotting the debug print output of
//! a type.  Additionally if the `serialization` feature is enabled the
//! `assert_serialized_snapshot_matches!` macro becomes available which
//! serializes an object with `serde` to yaml before snapshotting.
//!
//! Snapshots are stored in the `snapshots` folder right next to the test file
//! where this is used.  The name of the file is `<module>__<name>.snap` where
//! the `name` of the snapshot has to be provided to the assertion macro.
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
//! Created: 2019-01-13T22:16:48.669496+00:00
//! Creator: insta@0.1.0
//! Source: tests/test_snapshots.rs
//!
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
#[macro_use]
mod macros;
mod runtime;
#[cfg(test)]
mod test;

#[cfg(feature = "serialization")]
mod redaction;
#[cfg(feature = "serialization")]
mod serialization;

pub use crate::runtime::Snapshot;

// this should eventually become public api but probably somewhere else
#[doc(hidden)]
#[cfg(feature = "serialization")]
pub use crate::redaction::Selector;
#[cfg(feature = "serialization")]
#[doc(hidden)]
pub use serde_yaml::{Mapping, Number, Sequence, Value};

#[doc(hidden)]
pub mod _macro_support {
    pub use crate::runtime::assert_snapshot;
    #[cfg(feature = "serialization")]
    pub use crate::serialization::{serialize_value, serialize_value_redacted};
}
