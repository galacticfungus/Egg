use std::collections::HashMap;
use crate::snapshots::types::{SnapshotId, Snapshot};
use crate::hash::Hash;
use crate::error::Error;

use super::SnapshotIndex;

type Result<T> = std::result::Result<T, Error>;

impl SnapshotIndex {
    pub fn init() -> SnapshotIndex {
        SnapshotIndex {
            hash_index: HashMap::new(),
        }
    }

    pub fn restore() -> SnapshotIndex {
        // TODO: Load any index state here
        SnapshotIndex {
            hash_index: HashMap::new(),
        }
    }

    // If a hash
    pub fn get_id(&self, hash: &Hash) -> Option<SnapshotId> {
        if let Some(id) = self.hash_index.get(hash) {
            return Some(id.clone());
        } else {
            return None;
        }
    }

    // Adds a hash to the index assuming that there is no index associated with it
    pub fn add_hash(&mut self, hash: Hash) {
        // If there is already a hash stored do nothing
        if let None = self.hash_index.get(&hash) {
            // Adding a hash to the index that is already in there does nothing
            self.hash_index.insert(hash.clone(), SnapshotId::NotLoaded(hash.clone()));
        }
        // If the hash is already considered loaded we have nothing to do
    }

    // Associates a hash with an index, will panic if the hash is already associated with an index
    pub fn associate_hash(&mut self, hash: Hash, index: usize) -> Result<()> {
        // If an index is already associated with this hash panic as its a bug
        if let Some(SnapshotId::Indexed(_, _)) = self.hash_index.get(&hash) {
            panic!("Associating a hash with an index that is already associated with an index");
            // This is really only a problem if the two indexes aren't the same but it still shouldn't happen
        }
        // let hash_clone = hash.clone();
        self.hash_index.insert(hash.clone(), SnapshotId::Indexed(index, hash));
        Ok(())
    }

    pub fn is_indexed(&self, hash: &Hash) -> bool {
        if let Some(SnapshotId::Indexed(_, _)) = self.hash_index.get(hash) {
            return true;
        }
        return false;
    }
    /// Takes a snapshot and parses it for any information that will be useful to the index, such as adding its parent and children to the index as well as
    /// associating an index with this snapshots hash
    pub fn index_snapshot(&mut self, snapshot: &Snapshot, index: usize) -> Result<()> {
        // Add ID's of parents and children to index
        if let Some(parent_hash) = snapshot.get_parent() {
            self.add_hash(parent_hash.clone());
        }
        // Add Child ID's to the index
        for child in snapshot.get_children() {
            self.add_hash(child.clone());
        }
        if let Err(error) = self.associate_hash(snapshot.get_hash().clone(), index) {
            unimplemented!();// Its probably fine to just return an error from the function directly here
        }
        Ok(())
    }
}

// TODO: Test the above