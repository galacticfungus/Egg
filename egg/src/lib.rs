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
pub use crate::snapshots::types::SnapshotId;
pub use atomic::AtomicUpdate;