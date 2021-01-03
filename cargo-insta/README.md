<div align="center">
 <img src="https://github.com/mitsuhiko/insta/blob/master/assets/logo.png?raw=true" width="250" height="250">
 <p><strong>cargo-insta: review tool for insta, a snapshot testing library for Rust</strong></p>
</div>

`cargo-insta` provides a cargo command for [insta](https://github.com/mitsuhiko/insta)
snapshot reviews.

```
$ cargo install cargo-insta
$ cargo insta --help
```

## Commands

`cargo-insta` provides a few different commands to interact with insta snapshots.

### `review`

This is the main command you are likely to use.  It starts the interactive
review process.  It takes similar arguments to the other commands and typically
does not require any.  It will auto discover snapshots in the current workspace.
If you want to change the location you can use `--workspace-root` which
explicitly sets the path to the workspace or `--manifest-path` to set the
path to a specific `Cargo.toml`.

Once the review process is starting you can accept changes with `a`, reject
changes with `j` and skip with `s`.

### `test`

This is a special command that works exactly like `cargo test` but it will
force all snapshots assertions to pass.  That way you can collect all snapshot
changes in one go and review them.  You can also combine this with `--accept`
to just accept all changes, `--accept-unseen` to accept all previously unseen
snapshots or `--review` to start a review process afterwards. Additionally
this commands supports `--delete-unreferenced-snapshots` to automatically
delete all unreferenced snapshots after the test run.

### `reject`

Like `review` but automatically rejects all snapshots.

### `accept`

Like `accept` but automatically accepts all snapshots.

### `pending-snapshots`

A utility command to emit information about pending snapshots.  This is useful
when you want to script `cargo-insta`.  For instance this is now the visual
studio code extension interfaces with insta.

## License and Links

- [Issue Tracker](https://github.com/mitsuhiko/insta/issues)
- [Documentation](https://docs.rs/insta/)
  [![License](https://img.shields.io/github/license/mitsuhiko/insta)](https://github.com/mitsuhiko/insta/blob/master/LICENSE)
