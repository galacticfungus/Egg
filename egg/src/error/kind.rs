use super::{ErrorContext, UnderlyingError};
use std::fmt;

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
    /// The repository seems to be in an invalid state, ie an operation was performed that resulted in a value that should have been impossible
    InvalidState(Option<UnderlyingError>, ErrorContext),
    /// An operation was attempted that is currently not valid, ie trying to take a snapshot based on the current working snapshot when there is no working snapshot
    InvalidOperation(Option<UnderlyingError>, ErrorContext),
}

// TODO: Both Display and Debug should handle the cases where error is none and display a different message
impl std::fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // TODO: Custom message based on the underlying cause of the error
        match self {
            ErrorKind::FileOperation(_, context) => {
                write!(f, "A file operation has failed\n{}", context)
            }
            ErrorKind::ParsingError(_, context) => write!(
                f,
                "Reading data from the repository has failed\n{}",
                context
            ),
            ErrorKind::WriteFailed(_, context) => {
                write!(f, "Writing data to the repository has failed\n{}", context)
            }
            ErrorKind::RepositoryNotFound(context) => {
                write!(f, "No repository was found\n{}", context)
            }
            ErrorKind::InvalidRepository(context) => write!(
                f,
                "On disk state of repository appears to be invalid\n{}",
                context
            ),
            ErrorKind::InvalidParameter(_, context) => {
                write!(f, "Invalid parameter was used\n{}", context)
            }
            ErrorKind::InvalidState(_, context) => write!(
                f,
                "In memory state of repository appeats to be invalid\n{}",
                context
            ),
            ErrorKind::InvalidOperation(_, context) => write!(
                f,
                "Attempted an operation that is not currently possible\n{}",
                context
            ),
        }
    }
}

impl std::fmt::Debug for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ErrorKind::FileOperation(error, context) => write!(
                f,
                "A file operation has failed\nThe system error was {:?},\n\n{}",
                error, context
            ),
            ErrorKind::ParsingError(error, context) => write!(
                f,
                "Reading data from the repository has failed\nThe error was {:?}\n{}",
                error, context
            ),
            ErrorKind::WriteFailed(error, context) => write!(
                f,
                "Writing data to the repository has failed\nThe underlying error was {:?}\n{}",
                error, context
            ),
            ErrorKind::RepositoryNotFound(context) => {
                write!(f, "No repository was found\n{}", context)
            }
            ErrorKind::InvalidRepository(context) => write!(
                f,
                "On disk data structures have become invalid\n{}",
                context
            ),
            ErrorKind::InvalidParameter(error, context) => write!(
                f,
                "An invalid parameter was used or created inside egg, error was {:?}\n{}",
                error, context
            ),
            ErrorKind::InvalidState(error, context) => write!(
                f,
                "In memory data structures have become invalid, error was {:?}\n{}",
                error, context
            ),
            ErrorKind::InvalidOperation(error, context) => write!(
                f,
                "Attempted an operation that is not currently possible, error was {:?}\n{}",
                error, context
            ),
        }
    }
}
