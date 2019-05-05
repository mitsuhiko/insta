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
#![allow(clippy::redundant_closure)]
mod cargo;
mod cli;
mod inline;

use console::style;

fn main() {
    if let Err(err) = cli::run() {
        let exit_code = if let Some(ref exit) = err.downcast_ref::<cli::QuietExit>() {
            exit.0
        } else {
            println!("{} {}", style("error:").red().bold(), err);
            1
        };
        std::process::exit(exit_code);
    }
}
