<div align="center">
 <img src="https://github.com/mitsuhiko/insta/blob/master/assets/logo.png?raw=true" width="250" height="250">
 <p><strong>cargo-insta: review tool for insta, a snapshot testing library for Rust</strong></p>
</div>

`cargo-insta` provides a cargo command for [insta](https://insta.rs/)
snapshot reviews.

```
$ cargo install cargo-insta
$ cargo insta --help
```

## Usage

`cargo-insta` provides a few different commands to interact with insta snapshots.

For running tests you can use the `test` command, for reviewing snapshots `review`.
The reviewing process is interactive and prompts for all changes identified.
If you want to skip reviewing you can use `accept` and `reject` directly.

For more information refer to the [documentation](https://insta.rs/docs/cli/).

## License and Links

- [Documentation](https://insta.rs/docs/cli/)
- [Issue Tracker](https://github.com/mitsuhiko/insta/issues)
- License: [Apache-2.0](https://github.com/mitsuhiko/insta/blob/master/LICENSE)
