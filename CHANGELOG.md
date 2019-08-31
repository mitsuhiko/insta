# Changelog

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
