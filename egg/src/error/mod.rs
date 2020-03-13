mod base;
mod context;
mod kind;
mod underlying;

pub use base::Error as Error;
pub use kind::ErrorKind as ErrorKind;
pub(crate) use underlying::UnderlyingError;
pub(crate) use context::ErrorContext as ErrorContext;
use context::MessageType as MessageType;