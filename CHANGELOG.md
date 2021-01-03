# Changelog

## 1.5.0

* Add `pending-snapshots` parameter to `cargo-insta`.
* `cargo-insta` now honors ignore files.  This can be overridden
  with `--no-ignore`.
* `cargo-insta` now supports the vscode extension.

## 1.4.0

* Add `--delete-unreferenced-snapshots` parameter to `cargo-insta`.
* Switch to the `globset` crate for the `glob` feature.
* When `INSTA_UPDATE` is set to `always` or `unseen` it won't
  fail on execution.
* Changed informational outputs also show on pass.

## 1.3.0

* Expose more useful methods from `Content`.
* Fixes for latest rustc version.

## 1.2.0

* Fix invalid offset calculation for inline snapshot (#137)
* Added support for newtype variant redactions. (#139)

## 1.1.0

* Added the `INSTA_SNAPSHOT_REFERENCES_FILE` environment variable to support
  deletions of unreferenced snapshot files. (#136)
* Added support for TOML serializations.
* Avoid diff calculation on large input files. (#135)
* Added `prepend_module_to_snapshot` flag to disable prepending of module
  names to snapshot files. (#133)
* Made `console` dependency optional.  The `colors` feature can be disabled now
  which disables colored output.

## 1.0.0

* Globs now follow links (#132)
* Added CSV Support (#134)
* Changed globs to also include directories not just files.
* Support snapshots outside source folder. (#70)
* Update RON to 0.6.

## 0.16.1

* Add `Settings::bind_async` when the `async` feature is enabled. (#121)
* Bumped `console` dependency to 0.11. (#124)
* Fixed incorrect path handling for `glob!`. (#123)
* Remove `cargo-insta` from workspace and add `Cargo.lock`. (#116)

## 0.16.0

* Made snapshot names optional for inline snapshots. (#106)
* Remove legacy macros. (#115)
* Made small improvements to cargo-insta's messaging and flags (#114)
* Added new logo.
* Added `glob` support. (#112)
* Made `MetaData` fields internal. (#111)

## 0.15.0

* Added test output control (`INSTA_OUTPUT` envvar). (#103)

## 0.14.0

* Dependency bump for `console` (lowers total dependency count)
* Change binary name to `cargo insta` in help pages.

## 0.13.1

* Added support for `INSTA_UPDATE=unseen` to write out unseen snapshots without review (#96)
* Added the `backtrace` feature which adds support for test name (and thus snapshot name)
  recovery from the backtrace if rust-test is not used in concurrent mode (#94, #98)

## 0.13

* Add support for deep wildcard matches (#92)
* Use module paths for test names (#87) 
* Do not emit useless indentations for empty lines (#88)

## 0.12

* Improve redactions support (#81)
* Deprecated macros are now hidden
* Reduce number of dependencies further.
* Added support for newtype struct redactions.
* Fixed bugs with recursive content operations (#80)

## 0.11

* redactions are now an optional feature that must be turned on to be used (`redactions`).
* RON format is now an optional feature that must be turned on to be used (`ron`).
* added support for sorting maps before serialization.
* added settings support.
* added support for overriding the snapshot path.
* correctly handle nested macros that might contain inline snapshots.
* use thread name as snapshot name for inline snapshots.
* use leading whitespace normalization for inline snapshots.
* removed `creator` and `created` field from snapshot metadata.
* removed the `_matches` suffix from all macros.
* added an `--accept` option to `cargo insta test`
* added `--force-update-snapshots` option to `cargo insta test`
* added `--jobs` and `--release` argument to `cargo insta test`.

To upgrade to the new insta macros and snapshot formats you can use
[`fastmod`](https://crates.io/crates/fastmod) and `cargo-insta` together:

    $ cargo install fastmod
    $ cargo install cargo-insta
    $ fastmod '\bassert_([a-z]+_snapshot)_matches!' 'assert_${1}!' -e rs --accept-all
    $ cargo insta test --all --force-update-snapshots --accept
