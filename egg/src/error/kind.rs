use std::fmt;
use super::{UnderlyingError, ErrorContext};

/// The error type that occurred in egg storage
pub enum ErrorKind {
    /// Failed to perform a file system operation such as creating, opening, renaming or copying a file
    FileOperation(Option<UnderlyingError>, ErrorContext),
    /// Failed to read data from the file system
    ParsingError(Option<UnderlyingError>, ErrorContext),
    /// A write to the file system failed to complete
    WriteFailed(Option<UnderlyingError>, ErrorContext),
    /// A Repository was not found at the given location
    RepositoryNotFound(ErrorContext),
    /// An error occured because the repository is invalid in some way, ie the repository is missing a file that should exist
    InvalidRepository(ErrorContext),
    /// The operation could not be completed because a parameter was considered invalid
    InvalidParameter(Option<UnderlyingError>, ErrorContext),
}

// TODO: Both Display and Debug should handle the cases where error is none and display a different message
impl std::fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // TODO: Custom message based on the underlying cause of the error
        match self {
            ErrorKind::FileOperation(_, context) => write!(f, "A file operation has failed\n{}", context),
            ErrorKind::ParsingError(_, context) => write!(f, "Reading data from the repository has failed\n{}", context),
            ErrorKind::WriteFailed(_, context) => write!(f, "Writing data to the repository has failed\n{}", context),
            ErrorKind::RepositoryNotFound(context) => write!(f, "No repository was found\n{}", context),
            ErrorKind::InvalidRepository(context) => write!(f, "Repository appears to be in an invalid state\n{}", context),
            ErrorKind::InvalidParameter(_, context) => write!(f, "Invalid parameter was passed or used inside egg\n{}", context),
        }
    }
}

impl std::fmt::Debug for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ErrorKind::FileOperation(error, context) => write!(f, "A file operation has failed\nThe system error was {:?},\n\n{}", error, context),
            ErrorKind::ParsingError(error, context) => write!(f, "Reading data from the repository has failed\nThe error was {:?}\n{}", error, context),
            ErrorKind::WriteFailed(error, context) => write!(f, "Writing data to the repository has failed\nThe underlying error was {:?}\n{}", error, context),
            ErrorKind::RepositoryNotFound(context) => write!(f, "No repository was found\n{}", context),
            ErrorKind::InvalidRepository(context) => write!(f, "Repository appears to be in an invalid state\n{}", context),
            ErrorKind::InvalidParameter(error, context) => write!(f, "An invalid parameter was used or created inside egg, error was {:?}\n{}", error, context),
        }
    }
}