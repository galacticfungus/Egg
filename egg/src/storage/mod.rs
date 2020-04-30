use std::path;

// Contains types that persist user data data to disk, ie the files being placed in the repository
pub(crate) mod local;            // Reads and writes user data to the repository
pub(crate) mod stream;          // Provides extensions to Read Write traits to allow easier reading and writing a basic egg data types

// Re-export most of these modules from here
#[derive(Debug)]
pub struct LocalStorage {
    path_to_file_storage: path::PathBuf,
}