# insta snapshots

This extension lets you better work with Rust's [insta snapshot](https://crates.io/crates/insta)
files.  It adds syntax highlighting and other improvements.

## Features

After loading the extension `.snap` snapshots from insta are picked up automatically
and syntax highlighted as if they are YAML files.  For RON snapshots some small
tweaks are applied:

![example screenshot](https://raw.githubusercontent.com/mitsuhiko/insta/master/vscode-insta/images/screenshot.png)

Additionally you can "jump to definition" by hitting "F12" on a snapshot assertion
macro.
