# insta

<div align="center">
 <img src="https://github.com/mitsuhiko/insta/blob/master/assets/logo.png?raw=true" width="250" height="250">
 <p><strong>insta: a snapshot testing library for Rust</strong></p>
</div>

## How it Operates

This crate exports multiple macros for snapshot testing:

- `assert_snapshot_matches!` for comparing basic string snapshots.
- `assert_debug_snapshot_matches!` for comparing `Debug` outputs of values.
- `assert_display_snapshot_matches!` for comparing `Display` outputs of values.
- `assert_yaml_snapshot_matches!` for comparing YAML serialized
  output of types implementing `serde::Serialize`.
- `assert_ron_snapshot_matches!` for comparing RON serialized output of
  types implementing `serde::Serialize`.
- `assert_json_snapshot_matches!` for comparing JSON serialized output of
  types implementing `serde::Serialize`.

Snapshots are stored in the `snapshots` folder right next to the test file
where this is used.  The name of the file is `<module>__<name>.snap` where
the `name` of the snapshot has to be provided to the assertion macro.  If
no name is provided the name is derived from the test name.

Additionally snapshots can also be stored inline.  In that case the
`cargo-insta` tool is necessary.  See [inline snapshots](#inline-snapshots)
for more information.

For macros that work with `serde::Serialize` this crate also permits
redacting of partial values.  See [redactions](#redactions) for more
information.

<img src="https://github.com/mitsuhiko/insta/blob/artwork/screencast.gif?raw=true" alt="">

## Example

Install `insta`:

Recommended way if you have `cargo-edit` installed:

```rust
$ cargo add --dev insta
```

Alternatively edit your `Cargo.toml` manually and add `insta` as manual
dependency.

And for an improved review experience also install `cargo-insta`:

```rust
$ cargo install cargo-insta
```

```rust
use insta::assert_debug_snapshot_matches;

#[test]
fn test_snapshots() {
    let value = vec![1, 2, 3];
    assert_debug_snapshot_matches!("snapshot_name", value);
}
```

(If you do not want to provide a name for the snapshot read about
[unnamed snapshots](#unnamed-snapshots).)

The recommended flow is to run the tests once, have them fail and check
if the result is okay.  By default the new snapshots are stored next
to the old ones with the extra `.new` extension.  Once you are satisifed
move the new files over.  You can also use `cargo insta review` which
will let you interactively review them:

```rust
$ cargo test
$ cargo insta review
```

For more information on updating see [Snapshot Updating].

[Snapshot Updating]: #snapshot-updating

## Snapshot files

The committed snapshot files will have a header with some meta information
that can make debugging easier and the snapshot:

```rust
---
created: "2019-01-21T22:03:13.792906+00:00"
creator: insta@0.3.0
expression: "&User{id: Uuid::new_v4(), username: \"john_doe\".to_string(),}"
source: tests/test_redaction.rs
---
[
    1,
    2,
    3
]
```

## Snapshot Updating

During test runs snapshots will be updated according to the `INSTA_UPDATE`
environment variable.  The default is `auto` which will write all new
snapshots into `.snap.new` files if no CI is detected.

`INSTA_UPDATE` modes:

- `auto`: the default. `no` for CI environments or `new` otherwise
- `always`: overwrites old snapshot files with new ones unasked
- `new`: write new snapshots into `.snap.new` files.
- `no`: does not update snapshot files at all (just runs tests)

When `new` is used as mode the `cargo-insta` command can be used to review
the snapshots conveniently:

```rust
$ cargo install cargo-insta
$ cargo test
$ cargo insta review
```

"enter" or "a" accepts a new snapshot, "escape" or "r" rejects,
"space" or "s" skips the snapshot for now.

For more information invoke `cargo insta --help`.

## Test Assertions

By default the tests will fail when the snapshot assertion fails.  However
if a test produces more than one snapshot it can be useful to force a test
to pass so that all new snapshots are created in one go.

This can be enabled by setting `INSTA_FORCE_PASS` to `1`:

```rust
$ INSTA_FORCE_PASS=1 cargo test --no-fail-fast
```

A better way to do this is to run `cargo insta test --review` which will
run all tests with force pass and then bring up the review tool:

```rust
$ cargo insta test --review
```

## Redactions

For all snapshots created based on `serde::Serialize` output `insta`
supports redactions.  This permits replacing values with hardcoded other
values to make snapshots stable when otherwise random or otherwise changing
values are involved.

Redactions can be defined as the third argument to those macros with
the syntax `{ selector => replacement_value }`.

The following selectors exist:

- `.key`: selects the given key
- `["key"]`: alternative syntax for keys
- `[index]`: selects the given index in an array
- `[]`: selects all items on an array
- `[:end]`: selects all items up to `end` (excluding, supports negative indexing)
- `[start:]`: selects all items starting with `start`
- `[start:end]`: selects all items from `start` to `end` (end excluding,
  supports negative indexing).
- `.*`: selects all keys on that depth

Example usage:

```rust
#[derive(Serialize)]
pub struct User {
    id: Uuid,
    username: String,
    extra: HashMap<String, String>,
}

assert_yaml_snapshot_matches!("user", &User {
    id: Uuid::new_v4(),
    username: "john_doe".to_string(),
    extra: {
        let mut map = HashMap::new();
        map.insert("ssn".to_string(), "123-123-123".to_string());
        map
    },
}, {
    ".id" => "[uuid]",
    ".extra.ssn" => "[ssn]"
});
```

## Unnamed Snapshots

All snapshot assertion functions let you leave out the snapshot name.  In
that case the snapshot name is derived from the test name.  This works
because the rust test runner names the thread by the test name and the
name is taken from the thread name.  In case your test spawns additional
threads this will not work and you will need to provide a name explicitly.

Additionally if you have multiple snapshot assertions per test name a
counter will be appended:

```rust
#[test]
fn test_something() {
    assert_snapshot_matches!("first value");
    assert_snapshot_matches!("second value");
}
```

This will create two snapshots: `something` for the first value and
`something-2` for the second value.  The leading `test_` prefix is removed
if the function starts with that name.

## Inline Snapshots

Additionally snapshots can also be stored inline.  In that case the format
for the snapshot macros is `assert_snapshot_matches!(reference_value, @"snapshot")`.
The leading at sign (`@`) indicates that the following string is the
reference value.  `cargo-insta` will then update that string with the new
value on review.

Example:

```rust
#[derive(Serialize)]
pub struct User {
    username: String,
}

assert_yaml_snapshot_matches!(User {
    username: "john_doe".to_string(),
}, @"");
```

After the initial test failure you can run `cargo insta review` to
accept the change.  The file will then be updated automatically.

License: Apache-2.0
