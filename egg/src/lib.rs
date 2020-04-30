mod error;
mod traits;
mod atomic;
mod hash;
mod storage;
mod working;
mod snapshots;
mod staging;
mod egg;

pub use crate::egg::Repository;
pub use crate::snapshots::SnapshotId;
pub use atomic::AtomicUpdate;

// Eggs architecture is currently based on one entry point, it need not be, for instance to take a snapshot, use a method Egg::take_snapshot
// This would mean having to revalidate the repository each time an operation is performed

// TODO: A recoverable error - ie a function return that is either a result or a response object, extension to Error trait, allows a user to specifically control how to respond to an error