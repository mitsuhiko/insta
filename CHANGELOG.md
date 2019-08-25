# Changelog

## 0.11

* redactions are now an optional feature that must be turned on to be used (`redactions`).
* make RON support optional.
* added support for sorting maps before serialization.
* added settings support.
* added support for overriding the snapshot path.
* added an `--accept` command to `cargo insta test`
* correctly handle nested macros that might contain inline snapshots.
* use thread name as snapshot name for inline snapshots.
