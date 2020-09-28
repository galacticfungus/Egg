use std::path;
use crate::hash::Hash;

mod id;
mod snapshot;
mod builder;
mod file;
mod location;

#[derive(Debug, Clone, Eq, Hash)]
pub enum SnapshotId {
    Located(Hash, SnapshotLocation),  // Snapshot is not loaded but we know its location
    NotLocated(Hash),                 // Snapshot supposedly exists and is referenced in the repository but we don't know its location
    Indexed(usize, Hash),             // Snapshot is loaded and indexed in the vector of snapshots
}

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub enum SnapshotLocation {
    Simple,     // The snapshot is stored in a single file located in the snapshot directory on the local repository
    // Packed(index, path) - 
    // NotLocal - Remote snapshot
}

pub struct SnapshotBuilder {
    message: Option<String>,
    id: Option<Hash>,
    files: Vec<FileMetadata>,
    children: Vec<Hash>,
    parent: Option<Hash>,
}

// We only need Clone and PartialEq when testing
#[cfg_attr(test, derive(Clone, PartialEq))]
#[derive(Debug)]
/// Represents all the information that is stored about a file being placed in a snapshot
pub struct FileMetadata {
    path: path::PathBuf,
    file_size: u64,
    modified_time: u128,
    hash: Hash,
}

#[cfg_attr(test, derive(Clone, PartialEq))]
#[derive(Debug)]
pub struct Snapshot {
    id: Hash,
    message: std::string::String,
    // FIXME: This actually needs to be a list since snapshots may be merged? and we want to track what snapshots were merged
    parent: Option<Hash>,               // The snapshot that this snapshot is based off
    children: Vec<Hash>,                // Snapshots that this snapshot serves as the basis
    files: Vec<FileMetadata>,           // Each path has a hash associated with it, in addition to a file size and a modification time
}

// TODO: Test both snapshot and FileMetadata functionality here