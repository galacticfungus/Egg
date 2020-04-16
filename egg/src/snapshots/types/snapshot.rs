use std::path;
use std::fmt::Display;

use crate::hash::Hash;

#[cfg_attr(test, derive(Clone, PartialEq))]
#[derive(Debug)]
/// Represents all the information that is stored about a file being placed in a snapshot
pub struct FileMetadata {
    path: path::PathBuf,
    file_size: u64,
    modified_time: u128,
    hash: Hash,
}

impl FileMetadata {
    pub fn new(hash: Hash, file_size: u64, path: path::PathBuf, modified_time: u128) -> FileMetadata {
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
        format!("{} - hash {}/{} bytes",metadata.path().display(), metadata.hash, metadata.file_size)
    }
}

#[cfg_attr(test, derive(Clone, PartialEq))]
#[derive(Debug)]
pub struct Snapshot {
    id: Hash,
    message: std::string::String,
    // FIXME: This actually needs to be a list since snapshots may be merged?
    parent: Option<Hash>,               // The snapshot that this snapshot is based off
    children: Vec<Hash>,                // Snapshots that this snapshot serves as the basis
    files: Vec<FileMetadata>,  // Each path has a hash associated with it, in addition to a file size and a modification time
}

// TODO: A snapshot must record its history and its ID be based on its history as well as current values

impl Display for Snapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Snapshot {}", self.id)
    }
}

impl Snapshot {
    /// Creates a new snapshot
    pub(crate) fn new(id: Hash, message: String, mut files: Vec<FileMetadata>, children: Vec<Hash>, parent: Option<Hash>) -> Snapshot {
        // Sort the hashes in order so that equivilant snapshots are not order dependant
        files.sort_unstable_by(|first_file, second_file| {
            first_file.path().cmp(&second_file.path())
        });
        Snapshot {
            id,
            message,
            parent,
            children,
            files,
        }
    }

    pub(crate) fn breakup(self) -> (Hash, Vec<Hash>, Vec<FileMetadata>, Option<Hash>, String) {
        let Snapshot {id, parent, children, files, message} = self;
        (id, children, files, parent, message)
    }
}

impl Snapshot {
    pub(crate) fn add_child(&mut self, hash: Hash) {
        self.children.push(hash);
    }
}

impl Snapshot {
    pub fn get_message(&self) -> &str {
        self.message.as_str()
    }

    pub fn get_hash(&self) -> &Hash {
        &self.id
    }

    pub fn get_parent(&self) -> Option<&Hash> {
        self.parent.as_ref()
    }

    pub fn get_files(&self) -> &[FileMetadata] {
        self.files.as_slice()
    }

    pub fn get_children(&self) -> &[Hash] {
        self.children.as_slice()
    }
}

#[cfg(test)]
use rand::{self, Rng};
#[cfg(test)]
use rand::distributions::{Distribution, Standard, Alphanumeric};
#[cfg(test)]
impl Distribution<Snapshot> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Snapshot {
        Snapshot {
            message: std::iter::repeat(())
                .map(|()| rng.sample(Alphanumeric))
                .take(12)
                .collect(),
            id: Hash::generate_random_hash(),
            parent: if rand::random() {
                    // No parent
                    None
                } else {
                    // Random parent
                    Some(Hash::generate_random_hash())
                },
            children: rng.sample_iter(&Standard)
                         .take(6)
                         .collect(),
            files: rng.sample_iter(&Standard)
                         .take(4)
                         .collect(),

        }
    }
}

#[cfg(test)]
impl Distribution<Hash> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Hash {
        // rng.gen()
        Hash::generate_random_hash()
    }
}

#[cfg(test)]
impl Distribution<FileMetadata> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> FileMetadata {
        FileMetadata {
            file_size: rng.gen(),
            hash: Hash::generate_random_hash(),
            modified_time: rng.gen(),
            path: path::PathBuf::new(),
        }
    }
}

#[cfg(test)]
impl Snapshot {
    pub fn create_test_snapshot() -> Snapshot {
        let mut rng = rand::thread_rng();
        rng.gen()
    }
}