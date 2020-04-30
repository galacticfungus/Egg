use std::fmt;
use super::{ErrorContext, MessageType, ErrorKind, UnderlyingError};


/// An error that occurred in egg-lib
pub struct Error {
    kind: ErrorKind,
}


// TODO: A recoverable error - ie a function return that is either a result or a response object, extension to Error trait


impl Error {
    // Create a parsing error
    pub fn parsing_error(caused_by: Option<UnderlyingError>) -> Error {
        Error {
            kind: ErrorKind::ParsingError(caused_by, ErrorContext::new()),
        }
    }
    // Create a writing error
    pub fn write_error(caused_by: Option<UnderlyingError>) -> Error {
        Error {
            kind: ErrorKind::WriteFailed(caused_by, ErrorContext::new()),
        }
    }
    // Create a file error
    pub fn file_error(caused_by: Option<UnderlyingError>) -> Error {
        // TODO: A file error can use the path that caused the problem in a message since it is passed into the error
        Error {
            kind: ErrorKind::FileOperation(caused_by, ErrorContext::new()),
        }
    }
    // Create a invalid parameter error
    pub fn invalid_parameter(caused_by: Option<UnderlyingError>) -> Error {
        Error {
            kind: ErrorKind::InvalidParameter(caused_by, ErrorContext::new()),
        }
    }

    pub fn repository_not_found() -> Error {
        Error {
            kind: ErrorKind::RepositoryNotFound(ErrorContext::new()),
        }
    }

    pub fn invalid_repository() -> Error {
        Error {
            kind: ErrorKind::InvalidRepository(ErrorContext::new()),
        }
    }

    pub fn invalid_state(caused_by: Option<UnderlyingError>) -> Error {
        Error {
            kind: ErrorKind::InvalidState(caused_by, ErrorContext::new()),
        }
    }

    pub fn invalid_operation(caused_by: Option<UnderlyingError>) -> Error {
        Error {
            kind: ErrorKind::InvalidOperation(caused_by, ErrorContext::new()),
        }
    }
}

impl Error {
    // TODO: Context should have a state associated with it, if it is for display, debug or both
    fn get_context_mut(&mut self) -> &mut ErrorContext {
        match self.kind {
            ErrorKind::ParsingError(_, ref mut context) => context,
            ErrorKind::WriteFailed(_, ref mut context) => context,
            ErrorKind::FileOperation(_, ref mut context) => context,
            ErrorKind::RepositoryNotFound(ref mut context) => context,
            ErrorKind::InvalidRepository(ref mut context) => context,
            ErrorKind::InvalidParameter(_, ref mut context) => context,
            ErrorKind::InvalidState(_, ref mut context) => context,
            ErrorKind::InvalidOperation(_, ref mut context) => context,
        }
    }

    fn get_context(&self) -> &ErrorContext {
        match self.kind {
            ErrorKind::ParsingError(_, ref context) => context,
            ErrorKind::WriteFailed(_, ref context) => context,
            ErrorKind::FileOperation(_, ref context) => context,
            ErrorKind::RepositoryNotFound(ref context) => context,
            ErrorKind::InvalidRepository(ref context) => context,
            ErrorKind::InvalidParameter(_, ref context) => context,
            ErrorKind::InvalidState(_, ref context) => context,
            ErrorKind::InvalidOperation(_, ref context) => context,
        }
    }

    pub fn add_debug_message<S: Into<String>>(mut self, message: S) -> Self {
        // TODO: Null Op when debug not set?
        let context = self.get_context_mut();
        context.add_message(message.into(), MessageType::Debug);
        self
    }

    pub fn add_user_message<S: Into<String>>(mut self, message: S) -> Self {
        let context = self.get_context_mut();
        context.add_message(message.into(), MessageType::User);
        self
    }

    pub fn add_generic_message<S: Into<String>>(mut self, message: S) -> Self {
        let context = self.get_context_mut();
        context.add_message(message.into(), MessageType::Both);
        self
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.kind).unwrap();
        write!(f, "{:?}", self.get_context())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.kind).unwrap();
        write!(f, "{:?}", self.get_context())
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        // Map Option<UnderlyingError> to Option<&(dyn std::error::Error + 'static)>
        match self.kind {
            ErrorKind::FileOperation(ref error, _) => error.as_ref().map(|e| e.get_error()),
            ErrorKind::ParsingError(ref error, _) => error.as_ref().map(|e| e.get_error()),
            ErrorKind::WriteFailed(ref error, _) => error.as_ref().map(|e| e.get_error()),
            ErrorKind::RepositoryNotFound(_) => None,
            ErrorKind::InvalidRepository(_) => None,
            ErrorKind::InvalidParameter(ref error, _) => error.as_ref().map(|e| e.get_error()),
            ErrorKind::InvalidState(ref error, _) => error.as_ref().map(|e| e.get_error()),
            ErrorKind::InvalidOperation(ref error, _) => error.as_ref().map(|e| e.get_error()),
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_basic_error() {
        unimplemented!();
    }

    
}