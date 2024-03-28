<div align="center">
 <img src="https://github.com/mitsuhiko/insta/blob/master/assets/logo.png?raw=true" width="250" height="250">
 <p><strong>cargo-insta: review tool for insta, a snapshot testing library for Rust</strong></p>
</div>

`cargo-insta` provides a cargo command for [insta](https://insta.rs/)
snapshot reviews.

Starting with `cargo-insta` 1.38.0 you can install prebuild binaries for many platforms, you can also always just install them with `cargo install` manually.

Unix:

```
curl -LsSf https://insta.rs/install.sh | sh
```

Windows:

```
powershell -c "irm https://insta.rs/install.ps1 | iex"
```

To install a specific version (in this case 1.38.0):

Unix:

```
curl -LsSf https://github.com/mitsuhiko/insta/releases/download/1.38.0/cargo-insta-installer.sh | sh
```

Windows:

```
powershell -c "irm https://github.com/mitsuhiko/insta/releases/download/1.38.0/cargo-insta-installer.ps1 | iex"
```

You can also manually download the binaries here:

- [aarch64-apple-darwin](https://github.com/mitsuhiko/insta/releases/latest/download/cargo-insta-aarch64-apple-darwin.tar.xz) (Apple Silicon macOS)
- [x86_64-apple-darwin](https://github.com/mitsuhiko/insta/releases/latest/download/cargo-insta-x86_64-apple-darwin.tar.xz) (Intel macOS)
- [x86_64-pc-windows-msvc](https://github.com/mitsuhiko/insta/releases/latest/download/cargo-insta-x86_64-pc-widows-msvc.zip) (x64 Windows)
- [x86_64-unknown-linux-gnu](https://github.com/mitsuhiko/insta/releases/latest/download/cargo-insta-x86_64-unknown-linux-gnu.tar.xz) (x64 Linux, GNU)
- [x86_64-unknown-linux-musl](https://github.com/mitsuhiko/insta/releases/latest/download/cargo-insta-x86_64-unknown-linux-musl.tar.xz) (x64 Linux, MUSL)

Alternatively you can manually build and install them. To install an old
version ensure to pass the `--locked` flag so that the `Cargo.lock`
file is honored:

```
$ cargo install cargo-insta --version 1.15.0 --locked
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
