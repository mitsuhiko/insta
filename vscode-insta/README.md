# insta snapshots

This extension lets you better work with Rust's [insta snapshot](https://crates.io/crates/insta)
files.  It adds syntax highlighting and other improvements.

## Features

The following features are currently available.

### Jump to Definition

After loading the extension you can "jump to definition" by hitting "F12" on
a snapshot assertion macro:

![jump to definition](https://raw.githubusercontent.com/mitsuhiko/insta/master/vscode-insta/images/jump-to-definition.gif)

### Pending Snapshots View

All pending snapshots are show in the sidebar if insta is used in your project:

![sidebar](https://raw.githubusercontent.com/mitsuhiko/insta/master/vscode-insta/images/view.png)

Clicking on a snapshot opens a diff view where you can accept and reject the
snapshot.  This also works for inline snapshots.  Additionally you can instruct
cargo insta to accept or reject all snapshots in one go.

### Accepting / Rejecting

Snapshots can be diffed, accepted and rejected right from within vscode.  This is available
through the following commands:

* "Compare Snapshots": opens a comparison view, also from the tree view.
* "Switch Between Snapshots": switches between current and new snapshot.
* "Accept New Snapshot": moves the new snapshot over the old snapshot.
* "Reject New Snapshot": rejects (deletes) the new snapshot.

### Syntax Highlighting

For all insta `.snap` snapshots from insta syntax highlighting is provided as if they are YAML files.  For RON snapshots some small
tweaks are applied to make them more pleasing to the eyes:

![example screenshot](https://raw.githubusercontent.com/mitsuhiko/insta/master/vscode-insta/images/screenshot.png)
