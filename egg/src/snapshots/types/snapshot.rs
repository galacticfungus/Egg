use std::fmt::Display;

use crate::hash::Hash;
use super::FileMetadata;
use super::Snapshot;

// TODO: A snapshot must record its history but how does this affect the id

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

    // TODO: This should be removed
    pub(crate) fn breakup(self) -> (Hash, Vec<Hash>, Vec<FileMetadata>, Option<Hash>, String) {
        let Snapshot {id, parent, children, files, message} = self;
        (id, children, files, parent, message)
    }

    pub(crate) fn add_child(&mut self, hash: Hash) {
        self.children.push(hash);
    }

    pub fn get_message(&self) -> &str {
        self.message.as_str()
    }

    pub fn get_hash(&self) -> &Hash {
        &self.id
    }
    // TODO: Since snapshot is available to consumers this should probably be a SnapshotId rather than the hashes
    // TODO: The hash functions can be renamed to parent_hash and only available inside the crate
    // TODO: We can always create an additional type like SnapshotTree that is able to self modify to load parents or children to its data structure all within a single borrow
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
use std::path;
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
    fn sample<R: Rng + ?Sized>(&self, _: &mut R) -> Hash {
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