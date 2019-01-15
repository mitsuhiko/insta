# Insta

<a href="https://crates.io/crates/insta"><img src="https://img.shields.io/crates/v/insta.svg" alt=""></a>

<div align="center">
  <img src="https://github.com/mitsuhiko/insta/blob/master/assets/logo.png?raw=true" width="250" height="250">
  <p><strong>insta: a snapshot testing library for Rust</strong></p>
</div>

## What is Snapshot Testing?

Snapshot testing is just a fancy term for asserting that something equals a
reference value in a test.  However the reference value is stored in a separate
file and support is provided to update and diff this value.

Insta supports snapshotting the output of the `Debug` or `serde::Serialize` trait:

```rust
use insta::assert_debug_snapshot_matches;

#[test]
fn test_snapshots() {
    let value = vec![1, 2, 3];
    assert_debug_snapshot_matches!("snapshot_name", value);
}
```

<img src="https://github.com/mitsuhiko/insta/blob/master/assets/insta.gif?raw=true">

## API

This crate exports two basic macros for snapshot testing:
`assert_snapshot_matches!` for comparing basic string snapshots and
`assert_debug_snapshot_matches!` for snapshotting the debug print output of a
type. Additionally if the serialization feature is enabled the
`assert_serialized_snapshot_matches!` macro becomes available which serializes an
object with serde to yaml before snapshotting.

Snapshots are stored in the `snapshots` folder right next to the test file
where this is used.  The name of the file is `<module>__<name>.snap` where
the `name` of the snapshot has to be provided to the assertion macro.

To update the snapshots export the `INSTA_UPDATE` environment variable
and set it to `1`.  The snapshots can then be committed.

* [Documentation](https://docs.rs/insta)
* [Crate](https://crates.io/crates/insta)

## Workflow

The recommended flow is to run the tests once, have them fail and check
if the result is okay.  Once you are satisifed run the tests again with
`INSTA_UPDATE` set to `1` and updates will be stored:

```
$ INSTA_UPDATE=1 cargo test
```

## License

Insta is licensed under the Apache 2 license.
