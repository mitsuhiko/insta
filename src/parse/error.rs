use std::{error, fmt, result};

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    FailedParsingYaml,
    YamlIsInvalidJson,
    UnexpectedDataType,
    NumberIsInvalidU32,
    MissingField,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::FailedParsingYaml => "Failed parsing the provided YAML text",
            Self::YamlIsInvalidJson => "The YAML text could not be represented as JSON",
            Self::UnexpectedDataType => "The present data type wasn't what was expected",
            Self::NumberIsInvalidU32 => "The provided number is not a valid u32",
            Self::MissingField => "A required field was missing",
        })
    }
}

impl error::Error for Error {}
