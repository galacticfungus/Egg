mod state;
mod index;
mod types;
mod file;
mod init;
mod take;
mod data;

use std::path;

pub use types::{Snapshot, SnapshotId, FileMetadata, SnapshotLocation};
use types::SnapshotBuilder;
use std::collections::HashMap;
use std::collections::VecDeque;

use crate::hash::Hash;

// TODO: Refactor snapshots so that it is self contained, ie If it needs to save a snapshot that is a method in RepsoitorySnapshots implemented in the file module
// TODO: No need for seperate index, index decides if a snapshot must be loaded from disk or is already loaded
// Defines a public interface for the snapshot system, used to store and load snapshots from the repository
#[derive(Debug)]
pub struct RepositorySnapshots {
    state: SnapshotsState,       // Reads, writes and stores the current state of the snapshot system
    snapshots: Vec<Snapshot>,
    index: HashMap<Hash, SnapshotId>,
}

/// Represents the state of the snapshot system, ie what is the latest snapshot, what are the root snapshots
// TODO: Move all the state fileIO stuff into storage, state can be all different states tracked
// TODO: impl Storable for storage state
#[derive(Debug)]
pub struct SnapshotsState {
    path_to_state_file: path::PathBuf,
    working_snapshot: Option<Hash>,   // Hash ID of the current snapshot
    latest_snapshot: Option<Hash>,    // Path to the recent snapshot file
    recent_snapshots: VecDeque<Hash>, // A list of recently accessed snapshots
    root_snapshots: Vec<Hash>, // The root snapshots in the repository, ie snapshots with no parent
                               // TODO: Add a usize to track the number of snapshots current stored in current_snapshot_file
                               // TODO: A separate file that tracks only global snapshot changes
    end_snapshots: Vec<Hash>,
}

#[cfg(test)]
mod tests {
    use testspace::TestSpace;
    use super::RepositorySnapshots;
    use super::SnapshotsState;
    use super::Snapshot;
    use crate::working::WorkingDirectory;
    use crate::hash::Hash;
    use crate::storage::LocalStorage;
    use super::SnapshotLocation;

    #[test]
    fn init_snapshots_test() {
        let ts = TestSpace::new();
        let ts2 = ts.create_child();
        let path_to_repository = ts2.get_path();
        let path_to_working = ts.get_path();
        // CHecks that we can both create and load the snapshot module
        RepositorySnapshots::new(path_to_repository, path_to_working).expect("Failed to init snapshots");
        RepositorySnapshots::load(path_to_working, path_to_repository).expect("Failed to load snapshots");
        let snapshots_dir = path_to_repository.join(RepositorySnapshots::SNAPSHOTS_PATH);
        let snapshots_state = snapshots_dir.join(SnapshotsState::STATE_FILE_NAME);
        assert!(snapshots_dir.exists());
        assert!(snapshots_state.exists());
    }

    #[test]
    fn load_snapshot_test() {
        // Setup
        let mut ts = TestSpace::new();
        let ts2 = ts.create_child();
        let file_list = ts.create_random_files(4, 4096);
        let path_to_repository = ts2.get_path();
        let path_to_working = ts.get_path();
        // This checks that repository snapshots can load a snapshot and store the results internally
        let mut files_to_snapshot = WorkingDirectory::create_metadata_list(file_list).expect("Failed to get metadata of files to snapshot");
        let message = String::from("Test Snapshot");
        let id = Hash::generate_snapshot_id(message.as_str(), files_to_snapshot.as_mut_slice());
        let snapshot = Snapshot::new(id.clone(), message, files_to_snapshot, Vec::new(), None);
        let mut snapshots = RepositorySnapshots::new(path_to_repository, path_to_working).expect("Failed to initialize snapshots");
        let path_to_file = path_to_repository.join(RepositorySnapshots::get_path()).join(snapshot.get_hash().to_string());
        RepositorySnapshots::write_simple_snapshot(&snapshot, path_to_file.as_path(), path_to_working).expect("Failed to write snapshot");
        let index = snapshots.load_snapshot(snapshot.get_hash(), SnapshotLocation::Simple, path_to_repository, path_to_working).expect("Failed to load snapshot");
        let loaded_snapshot = snapshots.snapshots.get(index).expect("Failed to retrieve loaded snapshot");
        assert_eq!(&snapshot, loaded_snapshot);
    }

    #[test]
    fn take_snapshot_test() {
        let mut ts = TestSpace::new();
        let ts2 = ts.create_child();
        let mut file_list = ts.create_random_files(4, 4096);
        let path_to_repository = ts2.get_path();
        let path_to_working = ts.get_path();
        
        let mut ss = RepositorySnapshots::new(path_to_repository, path_to_working).expect("Failed to init snapshot module");
        let ls = LocalStorage::initialize(path_to_repository).expect("Local file storage could not be init");
        let id = ss.take_snapshot(None, "Test Snapshot", file_list.clone(), path_to_repository, path_to_working, &ls)
            .expect("Failed to take snapshot");
        let snapshot = ss.snapshot_by_id(id, path_to_repository, path_to_working).expect("Failed to load snapshot");
        assert_eq!(snapshot.get_message(), "Test Snapshot");
        let mut file_list2: Vec<&std::path::Path> = snapshot.get_files().iter().map(|meta| meta.path()).collect();
        assert_eq!(file_list2.sort(), file_list.sort());
        // TODO: That the snapshot was correctly saved as all the tests only test in memory structures
    }

    #[test]
    fn take_snapshot_with_child_test() {
        let mut ts = TestSpace::new();
        let ts2 = ts.create_child();
        let file_list = ts.create_random_files(4, 4096);
        let path_to_repository = ts2.get_path();
        let path_to_working = ts.get_path();
        
        let mut ss = RepositorySnapshots::new(path_to_repository, path_to_working).expect("Failed to init snapshot module");
        let ls = LocalStorage::initialize(path_to_repository).expect("Local file storage could not be init");
        let parent_snapshot = ss.take_snapshot(None, "Test Snapshot", file_list.clone(), path_to_repository, path_to_working, &ls)
            .expect("Failed to take snapshot");
        let child_snapshot = ss.take_snapshot(Some(parent_snapshot), "Child Snapshot", file_list, path_to_repository, path_to_working, &ls).expect("Failed to take child snapshot");
        let child = ss.snapshot_by_id(child_snapshot, path_to_repository, path_to_working).expect("Failed to get child snapshot");
        assert_eq!(child.get_message(), "Child Snapshot");
        // TODO: That the snapshot was correctly saved as all the tests only test in memory structures
    }

    #[test]
    fn locate_hash_test() {
        let mut ts = TestSpace::new();
        let ts2 = ts.create_child();
        let mut file_list = ts.create_random_files(4, 4096);
        let path_to_repository = ts2.get_path();
        let path_to_working = ts.get_path();
        
        let mut ss = RepositorySnapshots::new(path_to_repository, path_to_working).expect("Failed to init snapshot module");
        // ss.locate_hash(hash, path_to_repository)
        
    }

    // #[test]
    // fn take_retrieve_snapshot_with_no_parent_test() {
    //     let mut ts = TestSpace::new();
    //     let ts2 = ts.create_child();
    //     let file_list = ts.create_random_files(4, 4096);
    //     let path_to_repository = ts2.get_path();
    //     let path_to_working = ts.get_path();
        
    //     let fs = LocalFileStorage::initialize(path_to_repository).expect("Failed to init fs");
    //     {
    //         let mut ss = RepositorySnapshots::initialize(path_to_repository, path_to_working).expect("Failed to Initialize Snapshot Storage");
    //         ss.take_snapshot(None, "A Message", file_list, path_to_repository, path_to_working, &fs).expect("Failed to take snapshot");
    //     }
    //     let ss = RepositorySnapshots::restore(path_to_working, path_to_repository).expect("Failed to restore");
    //     let latest = ss.get_latest_snapshot().expect("Didn't find a latest snapshot");
    //     eprintln!("Initial ID: {:?}", latest);
    //     eprintln!("Snapshot Storage: {:?}", ss);
    //     let snapshot = ss.get_snapshot_by_id(latest.clone(), path_to_repository, path_to_working).expect("Failed to retrieve snapshot");
    //     eprintln!("After 1 ID: {:?}", latest);
    //     assert_eq!(snapshot.get_message(), "A Message");
    //     let snapshot2 = ss.get_snapshot_by_id(latest.clone(), path_to_repository, path_to_working).expect("Failed to retrieve snapshot");
    //     eprintln!("After 2 ID: {:?}", latest);
    //     assert_eq!(snapshot2.get_message(), "A Message");
    //     assert_eq!(ss.snapshots.len(), 1);
    // }

    // #[test]
    // fn take_snapshot_with_parent_test() {
    //     // TODO: Need to test the parent resolution when the parent snapshot isn't loaded
    //     let mut ts = TestSpace::new();
    //     let ts2 = ts.create_child();
    //     let file_list = ts.create_random_files(4, 1024);
    //     let mut file_list2 = ts.create_random_files(2, 1024);
    //     let mut file_list3 = file_list.clone();
    //     file_list3.append(&mut file_list2);
        
    //     let path_to_repository = ts2.get_path();
    //     let path_to_working = ts.get_path();
        
    //     let fs = LocalFileStorage::initialize(path_to_repository).expect("Failed to init fs");
    //     {
    //         let mut ss = RepositorySnapshots::initialize(path_to_repository, path_to_working).expect("Failed to Initialize Snapshot Storage");
    //         let parent_id = ss.take_snapshot(None, "A parent snapshot", file_list, path_to_repository, path_to_working, &fs).expect("Failed to take snapshot");
    //         ss.take_snapshot(Some(parent_id), "A child snapshot", file_list3, path_to_repository, path_to_working, &fs).expect("Failed to take child snapshot");
    //     }
    //     let mut ss = RepositorySnapshots::restore(path_to_working, path_to_repository).expect("Failed to restore SnapshotStorage");
    //     let child_id = ss.get_latest_snapshot().expect("No latest snapshot was found");
    //     let child_snapshot = match ss.get_snapshot_by_id(child_id.clone(), path_to_repository, path_to_working) {
    //         Some(snapshot) => snapshot,
    //         None => {
    //             /*Load it*/
    //             let RepositorySnapshots {ref mut index, ref mut snapshots, ..} = ss;
    //             RepositorySnapshots::load_snapshot(snapshots, index, child_id.get_hash(), path_to_repository, path_to_working).expect("Failed to load child snapshot");
    //             ss.get_snapshot_by_id(child_id, path_to_repository, path_to_working).unwrap()
    //         },
    //     };
    //     // let child_snapshot = ss.get_snapshot_by_id(child_id, path_to_repository, path_to_working).expect("Failed to get child snapshot");//.expect("Child snapshot was not loaded");
    //     // TODO: Need a interface on the snapshot that returns an ID not a hash
    //     let result_parent = child_snapshot.get_parent().expect("Failed to get the parent snapshot");
    //     let parent_id = match ss.index.get_id(result_parent) {
    //         Some(parent_id) => parent_id,
    //         None => unimplemented!(),
    //     };
    //     // TODO: get_snapshot_by_id should return an id when the snapshot could not be loaded so that it can be used 
    //     // TODO: The problem with by value is that if the snapshot isn't loaded then we need a new id
    //     let parent_snapshot = match ss.get_snapshot_by_id(parent_id.clone(), path_to_repository, path_to_working) {
    //         Some(snapshot) => snapshot,
    //         None => {
    //             /*Load it*/
    //             let RepositorySnapshots {ref mut index, ref mut snapshots, ..} = ss;
    //             RepositorySnapshots::load_snapshot(snapshots, index, parent_id.get_hash(), path_to_repository, path_to_working).expect("Failed to load child snapshot");
    //             ss.get_snapshot_by_id(parent_id, path_to_repository, path_to_working).expect("Child snapshot was not loaded even after explicitly loading it")
    //         },
    //     };
        
    //     assert_eq!(parent_snapshot.get_message(), "A parent snapshot");
    // }

    // #[test]
    // fn get_latest_snapshot_test() {
    //     let mut ts = TestSpace::new();
    //     let ts2 = ts.create_child();
    //     let file_list = ts.create_random_files(4, 4096);
    //     let path_to_repository = ts2.get_path();
    //     let path_to_working = ts.get_path();
    //     let fs = LocalFileStorage::initialize(path_to_repository).expect("Failed to init fs");
    //     let mut ss = RepositorySnapshots::initialize(path_to_repository, path_to_working).expect("Failed to Initialize Snapshot Storage");
    //     let old = ss.take_snapshot(None, "A Message", file_list, path_to_repository, path_to_working, &fs).expect("Failed to take snapshot");
    //     let recent = ss.get_latest_snapshot().expect("Didn't find a latest snapshot");
    //     assert_eq!(old, recent);
    // }

    // #[test]
    // fn get_end_snapshots_test() {
    //     // TODO: Need to test the parent resolution when the parent snapshot isn't loaded
    //     let mut ts = TestSpace::new();
    //     let ts2 = ts.create_child();
    //     let file_list = ts.create_random_files(4, 1024);
    //     let mut file_list2 = ts.create_random_files(2, 1024);
    //     let mut file_list3 = file_list.clone();
    //     file_list3.append(&mut file_list2);
        
    //     let path_to_repository = ts2.get_path();
    //     let path_to_working = ts.get_path();
        
    //     let fs = LocalFileStorage::initialize(path_to_repository).expect("Failed to init fs");
    //     let mut ss = RepositorySnapshots::initialize(path_to_repository, path_to_working).expect("Failed to Initialize Snapshot Storage");
    //     let parent_id = ss.take_snapshot(None, "A parent snapshot", file_list, path_to_repository, path_to_working, &fs).expect("Failed to take snapshot");
    //     let child_id = ss.take_snapshot(Some(parent_id), "A child snapshot", file_list3, path_to_repository, path_to_working, &fs).expect("Failed to take child snapshot");
    //     let end_snapshots = ss.get_end_snapshots().expect("Failed to obtain end snapshots");
    //     assert_eq!(end_snapshots[0], child_id);
    // }

    // #[test]
    // fn get_root_snapshots_test() {
    //     let mut ts = TestSpace::new();
    //     let ts2 = ts.create_child();
    //     let file_list = ts.create_random_files(4, 1024);
    //     let mut file_list2 = ts.create_random_files(2, 1024);
    //     let mut file_list3 = file_list.clone();
    //     file_list3.append(&mut file_list2);
        
    //     let path_to_repository = ts2.get_path();
    //     let path_to_working = ts.get_path();
        
    //     let fs = LocalFileStorage::initialize(path_to_repository).expect("Failed to init fs");
    //     let mut ss = RepositorySnapshots::initialize(path_to_repository, path_to_working).expect("Failed to Initialize Snapshot Storage");
    //     let parent_id = ss.take_snapshot(None, "A parent snapshot", file_list, path_to_repository, path_to_working, &fs).expect("Failed to take snapshot");
    //     let root_snapshots = ss.get_root_snapshots().expect("Failed to obtain root snapshots");
    //     assert_eq!(root_snapshots[0], parent_id);
    // }

    #[test]
    fn get_mut_snapshot_by_hash_test() {
        unimplemented!()
    }

    #[test]
    fn get_snapshot_by_hash_test() {
        unimplemented!()
    }

    // TODO: Get end snapshots
    // TODO: Get root snapshots
}