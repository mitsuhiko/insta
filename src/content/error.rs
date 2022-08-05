use std::{error, fmt, result};

use super::Content;

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    InvalidStructField(Content),
    FailedParsingYaml,
    UnexpectedDataType,
    MissingField,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidStructField(content) => write!(f, "Invalid struct field: {:?}", content),
            Self::FailedParsingYaml => f.write_str("Failed parsing the provided YAML text"),
            Self::UnexpectedDataType => {
                f.write_str("The present data type wasn't what was expected")
            }
            Self::MissingField => f.write_str("A required field was missing"),
        }
    }
}

impl error::Error for Error {}
