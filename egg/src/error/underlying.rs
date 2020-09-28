use std::fmt;
use std::io;
use std::string;

#[derive(Debug)]
pub enum UnderlyingError {
    Io(io::Error),
    InvalidString(string::FromUtf8Error),
    FailedConversion(std::num::TryFromIntError),
    // TODO: Path fail may need to be more generic than this
    PathFail(std::path::StripPrefixError),
}

impl UnderlyingError {
    pub fn get_error(&self) -> &(dyn std::error::Error + 'static) {
        match self {
            UnderlyingError::InvalidString(error) => error,
            UnderlyingError::Io(error) => error,
            UnderlyingError::FailedConversion(error) => error,
            UnderlyingError::PathFail(error) => error,
        }
    }
}

impl From<string::FromUtf8Error> for UnderlyingError {
    fn from(error: string::FromUtf8Error) -> UnderlyingError {
        UnderlyingError::InvalidString(error)
    }
}

impl From<io::Error> for UnderlyingError {
    fn from(error: io::Error) -> UnderlyingError {
        UnderlyingError::Io(error)
    }
}

impl From<std::num::TryFromIntError> for UnderlyingError {
    fn from(error: std::num::TryFromIntError) -> UnderlyingError {
        UnderlyingError::FailedConversion(error)
    }
}

impl From<std::path::StripPrefixError> for UnderlyingError {
    fn from(error: std::path::StripPrefixError) -> UnderlyingError {
        UnderlyingError::PathFail(error)
    }
}

impl std::fmt::Display for UnderlyingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnderlyingError::InvalidString(error) => write!(f, "{}", error),
            UnderlyingError::Io(error) => write!(f, "{}", error),
            UnderlyingError::FailedConversion(error) => write!(f, "{}", error),
            UnderlyingError::PathFail(error) => write!(f, "{}", error),
        }
    }
}
