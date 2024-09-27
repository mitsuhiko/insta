use std::error::Error;
use std::fmt;

use cargo_metadata::MetadataCommand;
use lazy_static::lazy_static;
use semver::Version;

/// Close without message but exit code.
#[derive(Debug)]
pub(crate) struct QuietExit(pub(crate) i32);

impl Error for QuietExit {}

impl fmt::Display for QuietExit {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", &self.0)
    }
}

#[derive(Debug)]
struct ErrMsg(String);

impl Error for ErrMsg {}

impl fmt::Display for ErrMsg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", &self.0)
    }
}

pub(crate) fn err_msg<S: Into<String>>(s: S) -> Box<dyn Error> {
    Box::new(ErrMsg(s.into()))
}

/// The insta version in the current workspace (i.e. not the `cargo-insta`
/// binary that's running).
fn read_insta_version() -> Result<Version, Box<dyn std::error::Error>> {
    MetadataCommand::new()
        .exec()?
        .packages
        .iter()
        .find(|package| package.name == "insta")
        .map(|package| package.version.clone())
        .ok_or("insta not found in cargo metadata".into())
}

lazy_static! {
    pub static ref INSTA_VERSION: Version = read_insta_version().unwrap();
}

/// `cargo-insta` version
// We could put this in a lazy_static
pub(crate) fn cargo_insta_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
