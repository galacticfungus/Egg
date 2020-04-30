use std::path;

use crate::snapshots::types::{SnapshotId, Snapshot, SnapshotLocation};
use crate::hash::Hash;
use crate::error::Error;

use super::RepositorySnapshots;

impl RepositorySnapshots {
    pub const fn get_path() -> &'static str {
        "snapshots"
    }

    // Note: The index only works with Hash's ID's are only used outside of egg
    pub fn init_index(&self) {

    }

    pub fn restore_index(&self) {
        // TODO: Load index state here, this should be included in the base snapshots restore
    }

    pub fn is_indexed(&self, hash: &Hash) -> bool {
        // Has the index seen the hash
        if let Some(id) = self.index.get(hash) {
            match id {
                // We only care about indexed
                SnapshotId::Indexed(_, _) => return true,
                _ => return false,
            }
        }
        false
    }

    // Retrieve the SnapshotId for a given hash
    pub fn get_id(&self, hash: &Hash, path_to_repository: &path::Path) -> Result<SnapshotId, Error> {
        // If the hash has an entry in the index then we can return
        if let Some(id) = self.index.get(hash) {
            return Ok(id.clone());
        } else {
            // Th problem with returning a simple NotLocated here is that the snapshot may not exist so we need to locate the hash to make sure it does exist or return an error
            let location = self.locate_hash(hash, path_to_repository)
                .ok_or_else(|| Error::invalid_parameter(None)
                    .add_generic_message("An attempt to retrieve a snapshot id failed as the hash was not found in the repository"))?;
            return Ok(SnapshotId::Located(hash.clone(), location));
        }
    }

    // Given a hash returns a location that describes where the Snapshot can be loaded from
    pub fn locate_hash(&self, hash: &Hash, path_to_repository: &path::Path) -> Option<SnapshotLocation> {
        // First check the index - this should include remote upstream repositories
        // Then check for simple ie hash exists as a file in repository/snapshots
        // Finally return None if we cant find the location
        let file_name = hash.to_string();
        let path_to_snapshot = path_to_repository.join(Self::get_path()).join(file_name);
        if path_to_snapshot.exists() {
            Some(SnapshotLocation::Simple)
        } else {
            None
        }
    }

    // TODO: Private function - all other methods must use an ID?
    pub fn snapshot_by_hash_mut(&mut self, hash: &Hash, path_to_repository: &path::Path, path_to_working: &path::Path) -> Result<&mut Snapshot, Error> {
        match self.get_id(hash, path_to_repository)? {
            // The hash exists and is loaded
            SnapshotId::Indexed(index, _) => {
                // Get snapshot from vector
                // Note: Index must be valid or the repository is in an invalid state
                debug_assert!(index < self.snapshots.len() - 1, "Snapshot vector is invalid and the get unchecked would have undefined");
                Ok(&mut self.snapshots[index])
            },
            // The hash exists but is not loaded
            SnapshotId::Located(hash, location) => {
                // Attempt to load the snapshot since we know its location
                let index = self.load_snapshot(&hash, location, path_to_repository, path_to_working)
                    .map_err(|err| err
                        .add_generic_message("While retrieving a snapshot by hash"))?;
                Ok(&mut self.snapshots[index])
            },
            SnapshotId::NotLocated(hash) => {
                let location = self.locate_hash(&hash, path_to_repository)
                    .ok_or(Error::invalid_parameter(None)
                        .add_generic_message("Hash could not be located even thogh it was referenced in a previous snapshot"))?;
                let index = self.load_snapshot(&hash, location, path_to_repository, path_to_working)
                    .map_err(|err| err
                        .add_generic_message("While retrieving a snapshot"))?;
                Ok(&mut self.snapshots[index])
            },
        }
    }

    // TODO: Unused?
    pub fn snapshot_by_hash(&mut self, hash: &Hash, path_to_repository: &path::Path, path_to_working: &path::Path) -> Result<&Snapshot, Error> {
        // Get the ID of the hash if one exists
        match self.get_id(hash, path_to_repository)? {
            // The hash exists and is loaded
            SnapshotId::Indexed(index, _) => {
                // Get snapshot from vector
                // Note: Index must be valid or the repository is in an invalid state
                debug_assert!(index < self.snapshots.len() - 1, "Snapshot vector is invalid");
                // The below is safe since the snapshots vector is never modified except for adding a new entry
                Ok(&self.snapshots[index])
            },
            // The hash exists but is not loaded
            SnapshotId::Located(hash, location) => {
                // Attempt to load the snapshot since we know its location
                let index = self.load_snapshot(&hash, location, path_to_repository, path_to_working)
                    .map_err(|err| err
                        .add_generic_message("While retrieving a snapshot by hash"))?;
                // The below is safe since we just added that item to the vector, if there was a problem then loading the snapshot would have returned the error
                Ok(&self.snapshots[index])
            },
            SnapshotId::NotLocated(hash) => {
                let location = self.locate_hash(&hash, path_to_repository)
                    .ok_or(Error::invalid_parameter(None)
                        .add_generic_message("Hash could not be located even thogh it was referenced in a previous snapshot"))?;
                let index = self.load_snapshot(&hash, location, path_to_repository, path_to_working).map_err(|err| err.add_generic_message("While retrieving a snapshot"))?;
                // The below is safe since we just added that item to the vector, if there was a problem then locating or loading the snapshot would have returned the error
                Ok(&self.snapshots[index])
            },
        }
    }

    /// Retrieves a snapshot from the repository, the ID is consumed to help prevent using an old ID
    pub fn snapshot_by_id(&mut self, id: SnapshotId, path_to_repository: &path::Path, path_to_working: &path::Path) -> Result<&Snapshot, Error> {
        // We don't trust the ID being used as it may be old - ie refers to an unloaded snapshot when the snapshot was already loaded
        let recent_id = self.get_id(id.get_hash(), path_to_repository)?;
        // If the snapshot is loaded then return its index, we can trust the index since items are never unloaded
        if let SnapshotId::Indexed(index, _) = recent_id {
            return Ok(&self.snapshots[index]);
        }
        // The ID is a hash but check if that hash has been loaded, this is the same as Some(SnapshotId::NotLoaded(hash) = hash)
        else {
            let snapshot = match recent_id {
                SnapshotId::Located(hash, location) => {
                    let index = self.load_snapshot(&hash, location, path_to_repository, path_to_working)?;
                    &self.snapshots[index]
                },
                SnapshotId::NotLocated(hash) => {
                    let location = self.locate_hash(&hash, path_to_repository)
                        .ok_or(Error::invalid_parameter(None)
                            .add_generic_message("The hash that was part of an ID marked as Not Located could not be found"))?;
                    let index = self.load_snapshot(&hash, location, path_to_repository, path_to_working)?;
                    &self.snapshots[index]
                },
                _ => unreachable!("Indexed was already checked")
            };
            Ok(snapshot)
        }
    }
}

// TODO: Test the above