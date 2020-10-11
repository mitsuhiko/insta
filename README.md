# insta

<div align="center">
 <img src="https://github.com/mitsuhiko/insta/blob/master/assets/logo.png?raw=true" width="250" height="250">
 <p><strong>insta: a snapshot testing library for Rust</strong></p>
</div>

## What are snapshot tests

Snapshots tests (also sometimes called approval tests) are tests that
assert values against a reference value (the snapshot).  This is similar
to how `assert_eq!` lets you compare a value against a reference value but
unlike simple string assertions, snapshot tests let you test against complex
values and come with comprehensive tools to review changes.

Snapshot tests are particularly useful if your reference values are very
large or change often.

## What it looks like:

```rust
#[test]
fn test_hello_world() {
    insta::assert_debug_snapshot!(vec![1, 2, 3]);
}
```

Curious?  There is a screencast that shows the entire workflow: [watch the insta
introduction screencast](https://www.youtube.com/watch?v=rCHrMqE4JOY&feature=youtu.be).
Or if you're not into videos, read the [one minute introduction](#introduction).

## Introduction

Install `insta`:

Recommended way if you have `cargo-edit` installed:

```
$ cargo add --dev insta
```

Alternatively edit your `Cargo.toml` manually and add `insta` as manual
dependency.

And for an improved review experience also install `cargo-insta`:

```
$ cargo install cargo-insta
```

```rust
use insta::assert_debug_snapshot;

#[test]
fn test_snapshots() {
    assert_debug_snapshot!(vec![1, 2, 3]);
}
```

The recommended flow is to run the tests once, have them fail and check
if the result is okay.  By default the new snapshots are stored next
to the old ones with the extra `.new` extension.  Once you are satisifed
move the new files over.  To simplify this workflow you can use
`cargo insta review` which will let you interactively review them:

```
$ cargo test
$ cargo insta review
```

For more information on updating see [Snapshot Updating].

[Snapshot Updating]: #snapshot-updating

## How it operates

This crate exports multiple macros for snapshot testing:

- `assert_snapshot!` for comparing basic string snapshots.
- `assert_debug_snapshot!` for comparing `Debug` outputs of values.
- `assert_display_snapshot!` for comparing `Display` outputs of values.
- `assert_csv_snapshot!` for comparing CSV serialized output of
  types implementing `serde::Serialize`. (requires the `csv` feature)
- `assert_toml_snapshot!` for comparing TOML serialized output of
  types implementing `serde::Serialize`. (requires the `toml` feature)
- `assert_yaml_snapshot!` for comparing YAML serialized
  output of types implementing `serde::Serialize`.
- `assert_ron_snapshot!` for comparing RON serialized output of
  types implementing `serde::Serialize`. (requires the `ron` feature)
- `assert_json_snapshot!` for comparing JSON serialized output of
  types implementing `serde::Serialize`.

Snapshots are stored in the `snapshots` folder right next to the test file
where this is used.  The name of the file is `<module>__<name>.snap` where
the `name` of the snapshot.  Snapshots can either be explicitly named or the
name is derived from the test name.

Additionally snapshots can also be stored inline.  In that case the
[`cargo-insta`](https://crates.io/crates/cargo-insta) tool is necessary.
See [inline snapshots](#inline-snapshots) for more information.

For macros that work with `serde::Serialize` this crate also permits
redacting of partial values.  See [redactions](#redactions) for more
information.

## Snapshot files

The committed snapshot files will have a header with some meta information
that can make debugging easier and the snapshot:

```
---
expression: "vec![1, 2, 3]"
source: tests/test_basic.rs
---
[
    1,
    2,
    3
]
```

## Snapshot updating

During test runs snapshots will be updated according to the `INSTA_UPDATE`
environment variable.  The default is `auto` which will write all new
snapshots into `.snap.new` files if no CI is detected so that `cargo-insta`
can pick them up.  Normally you don't have to change this variable.

`INSTA_UPDATE` modes:

- `auto`: the default. `no` for CI environments or `new` otherwise
- `always`: overwrites old snapshot files with new ones unasked
- `unseen`: behaves like `always` for new snapshots and `new` for others
- `new`: write new snapshots into `.snap.new` files
- `no`: does not update snapshot files at all (just runs tests)

When `new` or `auto` is used as mode the `cargo-insta` command can be used
to review the snapshots conveniently:

```
$ cargo install cargo-insta
$ cargo test
$ cargo insta review
```

"enter" or "a" accepts a new snapshot, "escape" or "r" rejects,
"space" or "s" skips the snapshot for now.

For more information invoke `cargo insta --help`.

## Test assertions

By default the tests will fail when the snapshot assertion fails.  However
if a test produces more than one snapshot it can be useful to force a test
to pass so that all new snapshots are created in one go.

This can be enabled by setting `INSTA_FORCE_PASS` to `1`:

```
$ INSTA_FORCE_PASS=1 cargo test --no-fail-fast
```

A better way to do this is to run `cargo insta test --review` which will
run all tests with force pass and then bring up the review tool:

```
$ cargo insta test --review
```

## Named snapshots

All snapshot assertion functions let you leave out the snapshot name in
which case the snapshot name is derived from the test name (with an optional
leading `test_` prefix removed.

This works because the rust test runner names the thread by the test name
and the name is taken from the thread name.  In case your test spawns additional
threads this will not work and you will need to provide a name explicitly.
There are some situations in which rust test does not name or use threads.
In these cases insta will panic with an error.  The `backtrace` feature can
be enabled in which case insta will attempt to recover the test name from
the backtrace.

Explicit snapshot naming can also otherwise be useful to be more explicit
when multiple snapshots are tested within one function as the default
behavior would be to just count up the snapshot names.

To provide an explicit name provide the name of the snapshot as first
argument to the macro:

```rust
#[test]
fn test_something() {
    assert_snapshot!("first_snapshot", "first value");
    assert_snapshot!("second_snapshot", "second value");
}
```

This will create two snapshots: `first_snapshot` for the first value and
`second_snapshot` for the second value.  Without explicit naming the
snapshots would be called `something` and `something-2`.

## Test Output Control

Insta by default will output quite a lot of information as tests run.  For
instance it will print out all the diffs.  This can be controlled by setting
the `INSTA_OUTPUT` environment variable.  The following values are possible:

* `diff` (default): prints the diffs
* `summary`: prints only summaries (name of snapshot files etc.)
* `minimal`: like `summary` but more minimal
* `none`: insta will not output any extra information

## Redactions

**Feature:** `redactions`

For all snapshots created based on `serde::Serialize` output `insta`
supports redactions.  This permits replacing values with hardcoded other
values to make snapshots stable when otherwise random or otherwise changing
values are involved.  Redactions became an optional feature in insta
0.11 and can be enabled with the `redactions` feature.

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
- `.**`: performs a deep match (zero or more items).  Can only be used once.

Example usage:

```rust
#[derive(Serialize)]
pub struct User {
    id: Uuid,
    username: String,
    extra: HashMap<String, String>,
}

assert_yaml_snapshot!(&User {
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

It's also possible to execute a callback that can produce a new value
instead of hardcoding a replacement value by using the
[`dynamic_redaction`](fn.dynamic_redaction.html) function:

```rust
assert_yaml_snapshot!(&User {
    id: Uuid::new_v4(),
    username: "john_doe".to_string(),
}, {
    ".id" => dynamic_redaction(|value, _| {
        // assert that the value looks like a uuid here
        "[uuid]"
    }),
});
```

## Globbing

**Feature:** `glob`

Sometimes it can be useful to run code against multiple input files.
The easiest way to accomplish this is to use the `glob!` macro which
runs a closure for each input path that matches.  Before the closure
is executed the settings are updated to set a reference to the input
file and the appropriate snapshot suffix.

Example:

```rust
use std::fs;

glob!("inputs/*.txt", |path| {
    let input = fs::read_to_string(path).unwrap();
    assert_json_snapshot!(input.to_uppercase());
});
```

The path to the glob macro is relative to the location of the test
file.  It uses the [`globwalk`](https://crates.io/crates/globwalk) crate
for actual glob operations.

## Inline Snapshots

Additionally snapshots can also be stored inline.  In that case the format
for the snapshot macros is `assert_snapshot!(reference_value, @"snapshot")`.
The leading at sign (`@`) indicates that the following string is the
reference value.  `cargo-insta` will then update that string with the new
value on review.

Example:

```rust
#[derive(Serialize)]
pub struct User {
    username: String,
}

assert_yaml_snapshot!(User {
    username: "john_doe".to_string(),
}, @"");
```

After the initial test failure you can run `cargo insta review` to
accept the change.  The file will then be updated automatically.

## Features

The following features exist:

* `csv`: enables CSV support (`assert_csv_snapshot!`)
* `ron`: enables RON support (`assert_ron_snapshot!`)
* `toml`: enables TOML support (`assert_toml_snapshot!`)
* `redactions`: enables support for redactions
* `glob`: enables support for globbing (`glob!`)

## Settings

There are some settings that can be changed on a per-thread (and thus
per-test) basis.  For more information see [settings](struct.Settings.html).

## Legacy Snapshot Formats

With insta 0.11 the snapshot format was improved for inline snapshots.  The
old snapshot format will continue to be available but if you want to upgrade
them make sure the tests pass first and then run the following command
to force a rewrite of them all:

```
$ cargo insta test --accept --force-update-snapshots
```

## Deleting Unused Snapshots

Insta cannot detect unused snapshot files.  The reason for this is that
insta does not control the execution of the entirety of the tests so it
cannot spot which files are actually unreferenced.  However you can use
the `INSTA_SNAPSHOT_REFERENCES_FILE` environment variable to
instruct insta to append all referenced files into a list.  This can then
be used to delete all files not referenced.  For instance one could use
[`ripgrep`](https://github.com/BurntSushi/ripgrep) like this:

```
export INSTA_SNAPSHOT_REFERENCES_FILE="$(mktemp)"
cargo test
rg --files -lg '*.snap' "$(pwd)" | grep -vFf "$INSTA_SNAPSHOT_REFERENCES_FILE" | xargs rm
rm -f $INSTA_SNAPSHOT_REFERENCES_FILE
```

License: Apache-2.0
