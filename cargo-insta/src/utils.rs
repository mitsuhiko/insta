use std::error::Error;
use std::fmt;

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

/// `cargo-insta` version (i.e. the binary that's currently running).
// We could put this in a lazy_static
pub(crate) fn cargo_insta_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
