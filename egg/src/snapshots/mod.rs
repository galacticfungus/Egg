mod state;
mod index;
pub mod types;

use std::path;
use rand;

use state::StorageState;
use types::{Snapshot, SnapshotId, FileMetadata};
use crate::storage;
use crate::storage::LocalFileStorage;
use crate::storage::RepositoryStorage;
use crate::hash::Hash;
use crate::error::Error;
use crate::AtomicUpdate;
use crate::working::WorkingDirectory;
use index::SnapshotIndex;
use types::SnapshotBuilder;

type Result<T> = std::result::Result<T, Error>;

// Defines a public interface for the snapshot system
#[derive(Debug)]
pub(crate) struct RepositorySnapshots {
    state: StorageState,       // Reads, writes and stores the current state of the snapshot system
    index: SnapshotIndex,
    snapshots: Vec<Snapshot>,
}

impl RepositorySnapshots {
    /// Creates a new snapshot system in a repository that is being created
    pub(crate) fn initialize(base_path: &path::Path) -> Result<RepositorySnapshots> {
        // initialize state - this requires a path to an initial snapshot data file
        let state = StorageState::initialize(base_path)?;
        let index = SnapshotIndex::init();
        // Initialize the infrastructure used to perform atomic updates to multiple files at once
        AtomicUpdate::init(base_path)?;
        // TODO: Need to obtain a rough estimate of the initial size of the memory structures
        let snapshot_storage = RepositorySnapshots {
            state,
            snapshots: Vec::new(), // TODO: Get estimate of needed capacity beofre creating
            index,
        };
        Ok(snapshot_storage)
    }

    /// Loads the current state of the snapshot system from the repository
    pub(crate) fn restore(working_path: &path::Path, repository_path: &path::Path) -> Result<RepositorySnapshots> {
        // TODO: Check that the snapshot system was left in a valid state
        // Restore State
        let state = match StorageState::load_state(repository_path) {
            Ok(state) => state,
            Err(error) => unimplemented!(),
        };
        // Restore Data
        // TODO: Need to init seperately since we or may not init with working snapshot
        let mut snapshots = Vec::new();
        let mut index = SnapshotIndex::restore();
        // Load the working snapshot
        let mut storage = RepositorySnapshots {
            state,
            snapshots,
            index,
        };
        let RepositorySnapshots {state, snapshots, index} = &mut storage;
        if let Some(working_snapshot) = state.get_working_snapshot() {
            // TODO: We need to verify that the path exists and if it doesn't instead of loading an individual snapshot use the index to locate a grouped snapshot
            // TODO: Basically use the index to find the file
            // TODO: Before restoring a snapshot check that it isn't already in the index
            if let Err(error) = Self::load_snapshot(snapshots, index, working_snapshot, repository_path, working_path) {
                unimplemented!();
            }
        }
        // Load the latest snapshot
        if let Some(latest_snapshot) = state.get_latest_snapshot() {
            // Only load this snapshot if it isn't already loaded
            if index.is_indexed(latest_snapshot) == false {
                if let Err(error) = Self::load_snapshot(snapshots, index, latest_snapshot, repository_path, working_path) {
                    unimplemented!();
                }
            }
        }
        // Load recently used snapshots
        // TODO: How useful is this in reality - it means the last 10 used snapshots are pre loaded
        for hash in state.get_recent_snapshots() {
            // Only load this snapshot if it isn't already loaded
            if index.is_indexed(hash) == false {
                if let Err(error) = Self::load_snapshot(snapshots, index, hash, repository_path, working_path) {
                    unimplemented!();
                }
            }
        }
        // TODO: If the number of snapshots loaded is below a certain threshold then should we load more based on parents or children of snapshots loaded
        
        Ok(storage)
    }

    // Return a randomly named path to a file that doesn't already exist, of a given length and with an optional prefix
    pub(crate) fn create_unique_file_path(base_path: &path::Path, prefix: Option<&str>, suffix: Option<&str>, name_length: usize) -> path::PathBuf {
        use rand::Rng;
        loop {
            let random_file_name: String = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(name_length)
            .collect();
            let mut file_name = String::new();
            if let Some(prefix) = prefix {
                file_name.push_str(prefix);
            }
            file_name.push_str(random_file_name.as_str());
            if let Some(suffix) = suffix {
                file_name.push_str(suffix);
            }
            let temp_path = base_path.join(file_name);
            if temp_path.exists() == false {
                return temp_path;
            }
        }
    }

    // TODO: Need to check if there is any point in creating this snapshot - ie has something changed - this can be linked to tracking
    // TODO: Track files that have been snapshot
    // TODO: Change current snapshot to parent snapshot - current snapshot should be independent from snapshot creation
    /// Create a snapshot
    pub fn take_snapshot<S: Into<String>>(&mut self, parent_snapshot: Option<SnapshotId>, snapshot_message: S, files_to_snapshot: Vec<path::PathBuf>, path_to_repository: &path::Path, path_to_working: &path::Path, file_storage: &LocalFileStorage) -> Result<SnapshotId> {
        // TODO: Add support for inserting a snapshot in between other snapshots?
        // TODO: Add support for multiple children
        // TODO: All files being changed must be updated at the same time
        // TODO: Ensure the repository state is valid before reaching this point
        // TODO: Snapshot Builder
        if parent_snapshot.is_some() {
            return self.take_child_snapshot(parent_snapshot.unwrap(), snapshot_message.into(), files_to_snapshot, path_to_repository, path_to_working, file_storage);
        } else {
            return self.take_root_snapshot(snapshot_message.into(), files_to_snapshot, path_to_repository, path_to_working, file_storage);
        }

    }
    /// Take a snapshot that has no parent
    fn take_root_snapshot(&mut self, snapshot_message: String, files_to_snapshot: Vec<path::PathBuf>, path_to_repository: &path::Path, path_to_working: &path::Path, file_storage: &LocalFileStorage) -> Result<SnapshotId> {
        let mut builder = SnapshotBuilder::new();
        // Get the files that are going to be included in the snapshot
        match WorkingDirectory::create_metadata_list(files_to_snapshot) {
            Ok(snapshot_files) => builder.add_files(snapshot_files),
            Err(error) => unimplemented!(),
        }
        builder.set_message(snapshot_message.into());
        builder.change_parent(None);
        let snapshot = builder.build();
        let mut atomic = AtomicUpdate::new(path_to_repository);
        // Let atomic know about the snapshot file that needs to be created
        let path_to_snapshot = atomic.queue_create(String::from(snapshot.get_hash()));
        // Write the snapshot to disk
        if let Err(error) = RepositoryStorage::store(&snapshot, path_to_snapshot.as_path(), path_to_working, path_to_repository) {
            unimplemented!("Failed to write snapshot, error was {:?}", error);
        };
        // Process all the files that will be stored in this snapshot and prepare to store them into the repository
        file_storage.store_snapshot(&snapshot, &mut atomic);
        // Update state and queue file changes
        self.update_state_for_root_snapshot(&mut atomic, &snapshot, self.snapshots.len())?;
        self.finish_taking_snapshot(snapshot, atomic)
    }

    /// Take a snapshot that has a parent
    fn take_child_snapshot(&mut self, parent_snapshot: SnapshotId, snapshot_message: String, files_to_snapshot: Vec<path::PathBuf>, path_to_repository: &path::Path, path_to_working: &path::Path, file_storage: &LocalFileStorage) -> Result<SnapshotId> {
        let mut builder = SnapshotBuilder::new();
        let mut file_iterator = files_to_snapshot.iter();
        let files_valid = file_iterator.all(|path| {
            (path.exists() && path.is_absolute())
        });
        if files_valid == false {
            unimplemented!("Files were not valid, list of files were {:?}", files_to_snapshot.as_slice());
        }
        // TODO: Process snapshot file list to return file sizes and last modification time
        match WorkingDirectory::create_metadata_list(files_to_snapshot) {
            Ok(snapshot_files) => builder.add_files(snapshot_files),
            Err(error) => unimplemented!(),
        }
        builder.set_message(snapshot_message);
        let parent_hash = parent_snapshot.take_hash();
        builder.change_parent(Some(parent_hash.clone()));
        let snapshot = builder.build();
        let mut atomic = AtomicUpdate::new(path_to_repository);
        let path_to_snapshot = atomic.queue_create(String::from(snapshot.get_hash()));
        // Queue the snapshot to be written to disk
        if let Err(error) = RepositoryStorage::store(&snapshot, path_to_snapshot.as_path(), path_to_working, path_to_repository) {
            unimplemented!("Failed to write snapshot, error was {:?}", error);
        };
        // Process all the files that will be stored in this snapshot and prepare to store them into the repository
        file_storage.store_snapshot(&snapshot, &mut atomic);
        // The parent needs to be modified so that it knows about its new child
        println!("Modifying the parent snapshot");
        // TODO: Check for cyclic references - ie parent points to itself 
        let parent_snapshot = self.get_mut_snapshot_by_hash(&parent_hash, path_to_repository, path_to_working);
        println!("Parent snapshot is {}", parent_snapshot);
        // Adjust the snapshot record
        parent_snapshot.add_child(snapshot.get_hash().clone());
        // Queue the file replacement operation
        let path_to_parent_snapshot = atomic.queue_replace(parent_snapshot.get_hash());
        // Rewrite the snapshot
        println!("Rewriting snapshot {:?}", parent_snapshot);
        if let Err(error) = RepositoryStorage::store(parent_snapshot, path_to_parent_snapshot.as_path(), path_to_working, path_to_repository) {
            unimplemented!("Error: {}", error);
            // TODO: Complete error
        }
        println!("Finished rewriting parent");
        
        // Update state and queue file changes
        self.update_state_for_child_snapshot(&mut atomic, &snapshot, self.snapshots.len())?;
        self.finish_taking_snapshot(snapshot, atomic)
    }

    fn update_state_for_root_snapshot(&mut self, atomic: &mut AtomicUpdate, snapshot: &Snapshot, total_snapshots: usize) -> Result<()> {
        self.state.change_state(atomic, |state_data| {                            
            // This snapshot has no parent so add it as a root snapshot
            state_data.add_root_node(snapshot.get_hash().clone());
            state_data.add_end_node(snapshot.get_hash().clone());
            println!("Add as root hash");
            // Update latest snapshot
            state_data.set_latest_snapshot(Some(snapshot.get_hash().clone()));
            Ok(())
        })
    }

    fn update_state_for_child_snapshot(&mut self, atomic: &mut AtomicUpdate, snapshot: &Snapshot, total_snapshots: usize) -> Result<()> {
        self.state.change_state(atomic, |state_data| {            
            if let Some(parent) = snapshot.get_parent() {
                // Remove the parent as a end node, as long as we don't insert snapshots between snapshots this will work
                // TODO: If a second child is being added to a parent then this will fail since the parent node is not a end node already
                state_data.remove_end_node(parent); 
                state_data.add_end_node(snapshot.get_hash().clone());
            } else {
                // If this snapshot has no parent add it as a root snapshot
                state_data.add_root_node(snapshot.get_hash().clone());
                println!("Add as root hash");
            }
            // Update latest snapshot
            state_data.set_latest_snapshot(Some(snapshot.get_hash().clone()));
            Ok(())
        })
    }

    /// Updates the index of the snapshot, adds the snapshot to the list of loaded snapshots and lets AtomicUpdate know that all operations have been queued
    fn finish_taking_snapshot(&mut self, snapshot: Snapshot, atomic: AtomicUpdate) -> Result<SnapshotId> {
        // TODO: Move to its own function
        let RepositorySnapshots { ref mut snapshots, ref mut state, index: ref mut snapshot_index, } = self;
        // Update index
        snapshot_index.index_snapshot(&snapshot, snapshots.len())?;
        // Add snapshot to the vector
        snapshots.push(snapshot);
        // Write the changes to disk but only if all operations complete
        if let Err(error) = atomic.complete() {
            unimplemented!("Failed to complete atomic, {}", error);
        }
        let snapshot_hash = snapshots[snapshots.len() - 1].get_hash().clone(); // Index is reduced by one in both cases since the hash was already added to the index
        let id = SnapshotId::Indexed(snapshots.len() - 1, snapshot_hash);
        Ok(id)
    }

    // TODO: Private function - all other methods must use an ID?
    pub fn get_mut_snapshot_by_hash(&mut self, hash: &Hash, path_to_repository: &path::Path, path_to_working: &path::Path) -> &mut Snapshot {
        let RepositorySnapshots { ref mut snapshots, ref mut state, index: ref mut snapshot_index, } = self;
        match snapshot_index.get_id(hash) {
            Some(SnapshotId::Indexed(index, _)) => {
                // Get snapshot from vector
                snapshots.get_mut(index).unwrap()
            },
            Some(SnapshotId::NotLoaded(_)) => {
                if let Err(error) = RepositorySnapshots::load_snapshot(snapshots, snapshot_index, &hash, path_to_repository, path_to_working) {
                    unimplemented!();
                }
                match snapshot_index.get_id(hash) {
                    Some(SnapshotId::Indexed(parent_index, _)) => snapshots.get_mut(parent_index).unwrap(),
                    _ => unreachable!("A snapshot returned an unloaded ID after being loaded"),
                }
            },
            None => unreachable!("The hash of a snapshot's parent does not exist in the repository"),
        }
    }
    // TODO: Unused?
    pub fn get_snapshot_by_hash<'a>(&self, snapshot_index: &mut SnapshotIndex, snapshots: &'a mut Vec<Snapshot>, hash: &Hash, path_to_repository: &path::Path, path_to_working: &path::Path) -> &'a Snapshot {
        
        match snapshot_index.get_id(hash) {
            Some(SnapshotId::Indexed(index, _)) => {
                // Get snapshot from vector
                snapshots.get(index).unwrap()
            },
            Some(SnapshotId::NotLoaded(_)) => {
                if let Err(error) = RepositorySnapshots::load_snapshot(snapshots, snapshot_index, &hash, path_to_repository, path_to_working) {
                    unimplemented!();
                }
                match snapshot_index.get_id(hash) {
                    Some(SnapshotId::Indexed(parent_index, _)) => snapshots.get(parent_index).unwrap(),
                    _ => unreachable!("A snapshot returned an unloaded ID after being loaded"),
                }
            },
            None => unreachable!("The hash of a snapshot's parent does not exist in the repository"),
        }
    }

    /// Retrieves a snapshot from the repository, the ID is consumed to help prevent using an old ID
    pub fn get_snapshot_by_id(&self, id: SnapshotId, path_to_repository: &path::Path, path_to_working: &path::Path) -> Option<&Snapshot> {
        // TODO: Don't consume the ID, print a debug message that specifies an old ID was used
        // We don't trust the ID being used as it may be old - ie refers to an unloaded snapshot when the snapshot was already loaded
        // If the snapshot is loaded then return its index
        if let SnapshotId::Indexed(index, _) = id {
            println!("Returning index {} from array of {:?}", index, self.snapshots);
            return Some(&self.snapshots[index]);
        } // The ID is a hash but check if that hash has been loaded
        else if let Some(SnapshotId::Indexed(index, _)) = self.index.get_id(id.get_hash()) {
            // TODO: Panic in debug builds?
            // Snapshot is loaded but an unloaded ID was passed - so the ID is out of date
            return Some(&self.snapshots[index]);
        } else {
            // Snapshot is not loaded so return None
            // TODO: This is almost an error, an ID was passed in 
            return None;
        }
    }

    /// Loads a snapshot from storage by using the index to locate it, it also updates the index so that it points to the new loaded snapshot
    // TODO: Rename this as it really loads the snapshot and adds it to the vector
    pub fn load_snapshot(snapshots: &mut Vec<Snapshot>, index: &mut SnapshotIndex, hash: &Hash, path_to_repository: &path::Path, path_to_working: &path::Path) -> Result<()> {
        // TODO: Confirm that the snapshot is SnapshotId::Hash here before reading the snapshot
        
        let file_name = String::from(hash);
        let path_to_snapshot = path_to_repository.join(file_name);
        let snapshot = match RepositoryStorage::restore_snapshot(path_to_snapshot.as_path(), path_to_working) {
            Ok(snapshot) => snapshot,
            Err(error) => unimplemented!(),
        };
        if let Some(SnapshotId::Indexed(_,_)) = index.get_id(snapshot.get_hash()) {
            // If the hash is already indexed then the snapshot must already be in the index
            unreachable!("Snapshot already indexed when trying to index it again")
        }
        // Associate the hash with an index
        let snapshot_index = snapshots.len();
        // Add child and parent hashes to the index and associate this snapshot with an index
        if let Err(error) = index.index_snapshot(&snapshot, snapshot_index) {
            unimplemented!();
        }
        snapshots.push(snapshot);
        Ok(())
    }

    pub fn get_latest_snapshot(&self) -> Option<SnapshotId> {
        if let Some(hash) = self.state.get_latest_snapshot() {
            if let Some(id) = self.index.get_id(hash) {
                return Some(id.clone());
            } else {
                return Some(SnapshotId::NotLoaded(hash.clone()));
            }
        } else {
            None
        }
    }

    // Returns all the snapshots that have no parent - meaning that they start a chain of snapshots (ie start nodes)
    pub fn get_root_snapshots(&self) -> Result<Vec<SnapshotId>> {
        let mut root_ids = Vec::new();
        for root_hash in self.state.get_root_snapshots() {
            let id = match self.index.get_id(root_hash) {
                Some(root_hash) => root_hash,
                None => {
                    let error = Error::parsing_error(None)
                        .add_debug_message(format!("A root snapshot hash could not be found in the repository, this means the repository is somehow corrupt, hash was {}", root_hash))
                        .add_user_message("The repository appears to be corrupt, a hash was expected to exist but doesn't");
                    return Err(error);
                },
            };
            root_ids.push(id);
        }
        Ok(root_ids)
    }

    /// Returns all the snapshots that have no children - meaning that they are the end of a list of snapshots (ie end nodes)
    pub fn get_end_snapshots(&self) -> Result<Vec<SnapshotId>> {
        let mut end_ids = Vec::new();
        for end_hash in self.state.get_end_snapshots() {
            let id = match self.index.get_id(end_hash) {
                Some(end_hash) => end_hash,
                None => {
                    let error = Error::parsing_error(None)
                        .add_debug_message(format!("An end snapshot hash could not be found in the repository, this means the repositories snapshot state is corrupt, either a snapshot was removed without updating the hash or an invalid hash was added to the snapshot state, the missing hash was {}", end_hash))
                        .add_user_message("The repository appears to be corrupt, a hash was expected to exist but doesn't");
                    return Err(error);
                },
            };
            end_ids.push(id);
        }
        Ok(end_ids)
    }

    // pub fn iter(&self) -> impl Iterator<Item = &Hash> {
    //     Iter {
    //         storage: &self,
    //         root_nodes: self.state.get_root_snapshots().co,
    //     }
    // }
}

// pub struct Iter<'a> {
//     // Root Nodes
//     root_nodes: &'a [Hash],
//     storage: &'a SnapshotStorage,
// }

// impl<'a> Iterator for Iter<'a> {
//     type Item = &'a Hash;
//     fn next(&mut self) -> Option<&'a Hash> {
//         None
//     }
// }

#[cfg(test)]
mod tests {
    use testspace::{TestSpace};
    use super::RepositorySnapshots;
    use crate::snapshots::index::SnapshotIndex;
    use crate::snapshots::types::{Snapshot, SnapshotId};
    use crate::storage::LocalFileStorage;
    use crate::storage::RepositoryStorage;
    use crate::hash::Hash;
    use crate::working::WorkingDirectory;

    #[test]
    fn load_snapshot_test() {
        let mut snapshots = Vec::new();
        let mut index = SnapshotIndex::init();
        let mut ts = TestSpace::new();
        let ts2 = ts.create_child();
        let file_list = ts.create_random_files(4, 4096);
        let path_to_repository = ts2.get_path();
        let path_to_working = ts.get_path();
        
        let mut files_to_snapshot = WorkingDirectory::create_metadata_list(file_list).expect("Failed to get metadata of files to snapshot");
        let message = String::from("Test Snapshot");
        let id = Hash::generate_snapshot_id(message.as_str(), files_to_snapshot.as_mut_slice());
        let snapshot = Snapshot::new(id.clone(), message, files_to_snapshot, Vec::new(), None);
        {
            // let fs = LocalFileStorage::initialize(path_to_repository).expect("Failed to initialize Local File Storage");
            let path_to_snapshot = path_to_repository.join(String::from(id.clone()));
            RepositoryStorage::store(&snapshot, path_to_snapshot.as_path(), path_to_working, path_to_repository).expect("Failed to write snapshot");
        }
        RepositorySnapshots::load_snapshot(&mut snapshots, &mut index, &id, path_to_repository, path_to_working).expect("Failed to restore snapshot");
        assert_eq!(&snapshot, snapshots.last().unwrap());
        // Check that the index was updated to point to the new snapshot location
        let result_id = index.get_id(&id).unwrap();
        assert_eq!(result_id, SnapshotId::Indexed(0, id.clone()));
    }

    #[test]
    fn retrieve_snapshot_restore_repository_test() {
        let mut ts = TestSpace::new();
        let ts2 = ts.create_child();
        let file_list = ts.create_random_files(4, 4096);
        let path_to_repository = ts2.get_path();
        let path_to_working = ts.get_path();
        
        let mut files = WorkingDirectory::create_metadata_list(file_list).expect("Failed to get metadata of files being snapshot");
        let message = String::from("Test Snapshot");
        let id = Hash::generate_snapshot_id(message.as_str(), files.as_mut_slice());
        let snapshot = Snapshot::new(id.clone(), message, files, Vec::new(), None);
        {
            RepositorySnapshots::initialize(path_to_repository).expect("Failed to Initialize Snapshot Storage");
        }
        {
            let fs = LocalFileStorage::initialize(path_to_repository).expect("Failed to initialize Local File Storage");
            let path_to_snapshot = path_to_repository.join(String::from(id.clone()));
            RepositoryStorage::store(&snapshot, path_to_snapshot.as_path(), path_to_working, path_to_repository).expect("Failed to write snapshot");
        }
        let mut ss = RepositorySnapshots::restore(path_to_working, path_to_repository).expect("Failed to restore Snapshot Storage");
        let hash_id = SnapshotId::NotLoaded(id.clone());
        
        if let None = ss.get_snapshot_by_id(hash_id, path_to_repository, path_to_working) {
            let RepositorySnapshots { snapshots, index, .. } = &mut ss;
            super::RepositorySnapshots::load_snapshot(snapshots, index, &id, path_to_repository, path_to_working).expect("Failed to load snapshot");
        }
        let hash_id = SnapshotId::NotLoaded(id.clone());
        let result = ss.get_snapshot_by_id(hash_id, path_to_repository, path_to_working).expect("Failed to find snapshot after loading it");
        assert_eq!(&snapshot, result);
        // Check that the index was updated to point to the new snapshot location
        let result_id = ss.index.get_id(&id).unwrap();
        assert_eq!(result_id, SnapshotId::Indexed(0, id.clone()));
    }

    #[test]
    fn take_snapshot_test() {
        let mut ts = TestSpace::new().allow_cleanup(true);
        let ts2 = ts.create_child();
        let file_list = ts.create_random_files(4, 4096);
        let path_to_repository = ts2.get_path();
        let path_to_working = ts.get_path();
        
        let fs = LocalFileStorage::initialize(path_to_repository).expect("Failed to init fs");
        {
            let mut ss = RepositorySnapshots::initialize(path_to_repository).expect("Failed to Initialize Snapshot Storage");
            ss.take_snapshot(None, "A Message", file_list, path_to_repository, path_to_working, &fs).expect("Failed to take snapshot");
        }
        let ss = RepositorySnapshots::restore(path_to_working, path_to_repository).expect("Failed to restore");
        let latest = ss.state.get_latest_snapshot();
        assert_eq!(latest.is_some(), true);
        let snapshot = &ss.snapshots[0];
        assert_eq!(snapshot.get_message(), "A Message");
        // TODO: Need to assert that files being snapshot were stored
        // TODO: That the snapshot was correctly saved
    }

    #[test]
    fn take_retrieve_snapshot_with_no_parent_test() {
        let mut ts = TestSpace::new();
        let ts2 = ts.create_child();
        let file_list = ts.create_random_files(4, 4096);
        let path_to_repository = ts2.get_path();
        let path_to_working = ts.get_path();
        
        let fs = LocalFileStorage::initialize(path_to_repository).expect("Failed to init fs");
        {
            let mut ss = RepositorySnapshots::initialize(path_to_repository).expect("Failed to Initialize Snapshot Storage");
            ss.take_snapshot(None, "A Message", file_list, path_to_repository, path_to_working, &fs).expect("Failed to take snapshot");
        }
        let ss = RepositorySnapshots::restore(path_to_working, path_to_repository).expect("Failed to restore");
        let latest = ss.get_latest_snapshot().expect("Didn't find a latest snapshot");
        eprintln!("Initial ID: {:?}", latest);
        eprintln!("Snapshot Storage: {:?}", ss);
        let snapshot = ss.get_snapshot_by_id(latest.clone(), path_to_repository, path_to_working).expect("Failed to retrieve snapshot");
        eprintln!("After 1 ID: {:?}", latest);
        assert_eq!(snapshot.get_message(), "A Message");
        let snapshot2 = ss.get_snapshot_by_id(latest.clone(), path_to_repository, path_to_working).expect("Failed to retrieve snapshot");
        eprintln!("After 2 ID: {:?}", latest);
        assert_eq!(snapshot2.get_message(), "A Message");
        assert_eq!(ss.snapshots.len(), 1);
    }

    #[test]
    fn take_snapshot_with_parent_test() {
        // TODO: Need to test the parent resolution when the parent snapshot isn't loaded
        let mut ts = TestSpace::new();
        let ts2 = ts.create_child();
        let file_list = ts.create_random_files(4, 1024);
        let mut file_list2 = ts.create_random_files(2, 1024);
        let mut file_list3 = file_list.clone();
        file_list3.append(&mut file_list2);
        
        let path_to_repository = ts2.get_path();
        let path_to_working = ts.get_path();
        
        let fs = LocalFileStorage::initialize(path_to_repository).expect("Failed to init fs");
        {
            let mut ss = RepositorySnapshots::initialize(path_to_repository).expect("Failed to Initialize Snapshot Storage");
            let parent_id = ss.take_snapshot(None, "A parent snapshot", file_list, path_to_repository, path_to_working, &fs).expect("Failed to take snapshot");
            ss.take_snapshot(Some(parent_id), "A child snapshot", file_list3, path_to_repository, path_to_working, &fs).expect("Failed to take child snapshot");
        }
        let mut ss = RepositorySnapshots::restore(path_to_working, path_to_repository).expect("Failed to restore SnapshotStorage");
        let child_id = ss.get_latest_snapshot().expect("No latest snapshot was found");
        let child_snapshot = match ss.get_snapshot_by_id(child_id.clone(), path_to_repository, path_to_working) {
            Some(snapshot) => snapshot,
            None => {
                /*Load it*/
                let RepositorySnapshots {ref mut index, ref mut snapshots, ..} = ss;
                RepositorySnapshots::load_snapshot(snapshots, index, child_id.get_hash(), path_to_repository, path_to_working).expect("Failed to load child snapshot");
                ss.get_snapshot_by_id(child_id, path_to_repository, path_to_working).unwrap()
            },
        };
        // let child_snapshot = ss.get_snapshot_by_id(child_id, path_to_repository, path_to_working).expect("Failed to get child snapshot");//.expect("Child snapshot was not loaded");
        // TODO: Need a interface on the snapshot that returns an ID not a hash
        let result_parent = child_snapshot.get_parent().expect("Failed to get the parent snapshot");
        let parent_id = match ss.index.get_id(result_parent) {
            Some(parent_id) => parent_id,
            None => unimplemented!(),
        };
        // TODO: get_snapshot_by_id should return an id when the snapshot could not be loaded so that it can be used 
        // TODO: The problem with by value is that if the snapshot isn't loaded then we need a new id
        let parent_snapshot = match ss.get_snapshot_by_id(parent_id.clone(), path_to_repository, path_to_working) {
            Some(snapshot) => snapshot,
            None => {
                /*Load it*/
                let RepositorySnapshots {ref mut index, ref mut snapshots, ..} = ss;
                RepositorySnapshots::load_snapshot(snapshots, index, parent_id.get_hash(), path_to_repository, path_to_working).expect("Failed to load child snapshot");
                ss.get_snapshot_by_id(parent_id, path_to_repository, path_to_working).expect("Child snapshot was not loaded even after explicitly loading it")
            },
        };
        
        assert_eq!(parent_snapshot.get_message(), "A parent snapshot");
    }

    #[test]
    fn get_latest_snapshot_test() {
        let mut ts = TestSpace::new();
        let ts2 = ts.create_child();
        let file_list = ts.create_random_files(4, 4096);
        let path_to_repository = ts2.get_path();
        let path_to_working = ts.get_path();
        let fs = LocalFileStorage::initialize(path_to_repository).expect("Failed to init fs");
        let mut ss = RepositorySnapshots::initialize(path_to_repository).expect("Failed to Initialize Snapshot Storage");
        let old = ss.take_snapshot(None, "A Message", file_list, path_to_repository, path_to_working, &fs).expect("Failed to take snapshot");
        let recent = ss.get_latest_snapshot().expect("Didn't find a latest snapshot");
        assert_eq!(old, recent);
    }

    #[test]
    fn get_end_snapshots_test() {
        // TODO: Need to test the parent resolution when the parent snapshot isn't loaded
        let mut ts = TestSpace::new();
        let ts2 = ts.create_child();
        let file_list = ts.create_random_files(4, 1024);
        let mut file_list2 = ts.create_random_files(2, 1024);
        let mut file_list3 = file_list.clone();
        file_list3.append(&mut file_list2);
        
        let path_to_repository = ts2.get_path();
        let path_to_working = ts.get_path();
        
        let fs = LocalFileStorage::initialize(path_to_repository).expect("Failed to init fs");
        let mut ss = RepositorySnapshots::initialize(path_to_repository).expect("Failed to Initialize Snapshot Storage");
        let parent_id = ss.take_snapshot(None, "A parent snapshot", file_list, path_to_repository, path_to_working, &fs).expect("Failed to take snapshot");
        let child_id = ss.take_snapshot(Some(parent_id), "A child snapshot", file_list3, path_to_repository, path_to_working, &fs).expect("Failed to take child snapshot");
        let end_snapshots = ss.get_end_snapshots().expect("Failed to obtain end snapshots");
        assert_eq!(end_snapshots[0], child_id);
    }

    #[test]
    fn get_root_snapshots_test() {
        let mut ts = TestSpace::new();
        let ts2 = ts.create_child();
        let file_list = ts.create_random_files(4, 1024);
        let mut file_list2 = ts.create_random_files(2, 1024);
        let mut file_list3 = file_list.clone();
        file_list3.append(&mut file_list2);
        
        let path_to_repository = ts2.get_path();
        let path_to_working = ts.get_path();
        
        let fs = LocalFileStorage::initialize(path_to_repository).expect("Failed to init fs");
        let mut ss = RepositorySnapshots::initialize(path_to_repository).expect("Failed to Initialize Snapshot Storage");
        let parent_id = ss.take_snapshot(None, "A parent snapshot", file_list, path_to_repository, path_to_working, &fs).expect("Failed to take snapshot");
        let root_snapshots = ss.get_root_snapshots().expect("Failed to obtain root snapshots");
        assert_eq!(root_snapshots[0], parent_id);
    }

    #[test]
    fn get_mut_snapshot_by_hash_test() {
        unimplemented!()
    }

    #[test]
    fn get_snapshot_by_hash_test() {
        unimplemented!()
    }
}