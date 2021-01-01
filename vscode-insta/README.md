# insta snapshots

This extension lets you better work with Rust's [insta snapshot](https://crates.io/crates/insta)
files.  It adds syntax highlighting and other improvements.

## Features

After loading the extension you can "jump to definition" by hitting "F12" on
a snapshot assertion macro:

![jump to definition](https://raw.githubusercontent.com/mitsuhiko/insta/master/vscode-insta/images/jump-to-definition.gif)

For all insta `.snap` snapshots from insta syntax highlighting is provided as if they are YAML files.  For RON snapshots some small
tweaks are applied to make them more pleasing to the eyes:

![example screenshot](https://raw.githubusercontent.com/mitsuhiko/insta/master/vscode-insta/images/screenshot.png)
