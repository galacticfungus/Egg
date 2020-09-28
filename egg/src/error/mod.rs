mod base;
mod context;
mod kind;
mod underlying;

pub use base::Error;
pub(crate) use context::ErrorContext;
use context::MessageType;
pub use kind::ErrorKind;
pub(crate) use underlying::UnderlyingError;
