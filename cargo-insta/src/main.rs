//! <div align="center">
//!  <img src="https://github.com/mitsuhiko/insta/blob/master/assets/logo.png?raw=true" width="250" height="250">
//!  <p><strong>cargo-insta: review tool for insta, a snapshot testing library for Rust</strong></p>
//!</div>
//!
//! This crate provides a cargo command for insta snapshot reviews.
//!
//! ```ignore
//! $ cargo install cargo-insta
//! $ cargo insta --help
//! ```
//!
//! For more information see [the insta crate documentation](https://docs.rs/insta).
mod cargo;
mod cli;
mod inline;

use insta::ColorMap;

fn main() {
    if let Err(err) = cli::run() {
        let exit_code = if let Some(ref exit) = err.downcast_ref::<cli::QuietExit>() {
            exit.0
        } else {
            let cm = ColorMap::current();
            println!("{} {}", cm.failure("error:").bold(), err);
            1
        };
        std::process::exit(exit_code);
    }
}
