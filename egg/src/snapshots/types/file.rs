use super::FileMetadata;
use crate::hash::Hash;

use std::path;

impl FileMetadata {
    pub fn new(
        hash: Hash,
        file_size: u64,
        path: path::PathBuf,
        modified_time: u128,
    ) -> FileMetadata {
        FileMetadata {
            hash,
            path,
            file_size,
            modified_time,
        }
    }

    pub fn hash(&self) -> &Hash {
        &self.hash
    }

    pub fn filesize(&self) -> u64 {
        self.file_size
    }

    pub fn modified_time(&self) -> u128 {
        self.modified_time
    }

    pub fn path(&self) -> &path::Path {
        self.path.as_path()
    }
}

impl From<&FileMetadata> for String {
    fn from(metadata: &FileMetadata) -> Self {
        format!(
            "{} - hash {}/{} bytes",
            metadata.path().display(),
            metadata.hash,
            metadata.file_size
        )
    }
}
