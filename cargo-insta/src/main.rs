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
mod cargo;
mod cli;

fn main() {
    if let Err(err) = cli::run() {
        println!("error: {}", err);
        std::process::exit(1);
    }
}
