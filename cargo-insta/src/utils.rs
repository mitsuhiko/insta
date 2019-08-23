use std::error::Error;
use std::fmt;

/// Close without message but exit code.
#[derive(Debug)]
pub struct QuietExit(pub i32);

impl Error for QuietExit {}

impl fmt::Display for QuietExit {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", &self.0)
    }
}

#[derive(Debug)]
pub struct ErrMsg(String);

impl Error for ErrMsg {}

impl fmt::Display for ErrMsg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", &self.0)
    }
}

pub fn err_msg<S: Into<String>>(s: S) -> Box<dyn Error> {
    return Box::new(ErrMsg(s.into()));
}