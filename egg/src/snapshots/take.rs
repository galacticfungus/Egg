use super::{RepositorySnapshots, SnapshotBuilder, SnapshotId};
use crate::{working::WorkingDirectory, atomic::AtomicUpdate, error::Error, storage::LocalStorage};

use std::path;

impl RepositorySnapshots {
    // TODO: Need to check if there is any point in creating this snapshot - ie has something changed - this can be linked to tracking
    // TODO: Track files that have been snapshot
    // TODO: Change current snapshot to parent snapshot - current snapshot should be independent from snapshot creation
    /// Create a snapshot
    pub fn take_snapshot<S: Into<String>>(&mut self, parent_snapshot: Option<SnapshotId>, snapshot_message: S, files_to_snapshot: Vec<path::PathBuf>, path_to_repository: &path::Path, path_to_working: &path::Path, file_storage: &LocalStorage) -> Result<SnapshotId, Error> {
        // TODO: Add support for inserting a snapshot in between other snapshots?
        // TODO: Add support for multiple children
        // TODO: Ensure the repository state is valid before reaching this point
        if let Some(parent_snapshot) = parent_snapshot {
            return self.take_child_snapshot(parent_snapshot, snapshot_message.into(), files_to_snapshot, file_storage, path_to_working, path_to_repository);
        } else {
            return self.take_root_snapshot(snapshot_message.into(), files_to_snapshot, file_storage, path_to_working, path_to_repository);
        }
    }
    /// Take a snapshot that has no parent
    fn take_root_snapshot(&mut self, snapshot_message: String, files_to_snapshot: Vec<path::PathBuf>, file_storage: &LocalStorage, path_to_working: &path::Path, path_to_repository: &path::Path) -> Result<SnapshotId, Error> {
        let builder = SnapshotBuilder::new();
        // Get the files that are going to be included in the snapshot
        let snapshot_files = WorkingDirectory::create_metadata_list(files_to_snapshot)
            .map_err(|err| err
                .add_generic_message("Failed to create metadata list while taking a root snapshot"))?;
        let snapshot = builder.add_files(snapshot_files)
            .set_message(snapshot_message.into())
            .change_parent(None)
            .build();
        // Initialise the atomic updater
        let mut atomic = AtomicUpdate::load(path_to_working, path_to_repository);
        // TODO: Need to make this better
        let file_name = String::from(snapshot.get_hash());
        let snapshot_filename = path_to_repository.join(path::Path::new("snapshots")).join(file_name);
        let path_to_snapshot = atomic.queue_create(snapshot_filename)
            .map_err(|err| err
                .add_generic_message("While taking a root snapshot"))?;
        // Write the snapshot to disk
        // TODO: Again in most cases the snapshot being written will be simple but provisions need to be made to support packed files
        Self::write_simple_snapshot(&snapshot, path_to_snapshot.as_path(), path_to_working)?;
        // Process all the files that will be stored in this snapshot and prepare to store them into the repository
        file_storage.store_snapshot(&snapshot, &mut atomic)
            .map_err(|err| err
                .add_generic_message("while taking a root snapshot"))?;
        // Update state and queue file changes
        // Update state and queue state file changes
        self.update_state(&mut atomic, &snapshot, false)?;
        self.snapshots.push(snapshot);
        // Write all queued file changes to disk
        atomic.complete()
            .map_err(|err| err
                .add_generic_message("Failed to complete the atomic operation during a root snapshot"))?;
        let snapshot_hash = self.snapshots[self.snapshots.len() - 1].get_hash().clone(); // Index is reduced by one in both cases since the hash was already added to the index
        let id = SnapshotId::Indexed(self.snapshots.len() - 1, snapshot_hash);
        Ok(id)
    }

    /// Take a snapshot that has a parent
    fn take_child_snapshot(&mut self, parent_snapshot: SnapshotId, snapshot_message: String, files_to_snapshot: Vec<path::PathBuf>, file_storage: &LocalStorage, path_to_working: &path::Path, path_to_repository: &path::Path) -> Result<SnapshotId, Error> {
        let builder = SnapshotBuilder::new();
        let mut file_iterator = files_to_snapshot.iter();
        let files_valid = file_iterator.all(|path| {
            path.exists() && path.is_absolute()
        });
        if files_valid == false {
            unimplemented!("Files were not valid, list of files were {:?}", files_to_snapshot.as_slice());
        }
        // Get the list of files to include in the snapshot
        let snapshot_files = WorkingDirectory::create_metadata_list(files_to_snapshot)
            .map_err(|err| err
                .add_generic_message("Failed to create metadata list while taking a child snapshot"))?;
        let parent_hash = parent_snapshot.take_hash();
        // Create the child snapshot
        let snapshot = builder.add_files(snapshot_files)
                .set_message(snapshot_message)
                .change_parent(Some(parent_hash.clone()))
                .build();
        let path_to_snapshot = path_to_repository.join(RepositorySnapshots::get_path()).join(snapshot.get_hash().to_string());
        // TODO: Determine what type of snapshot we are taking and the location it is being saved, ie in most cases the snapshot will be simple but the parent may be packed
        let mut atomic = AtomicUpdate::load(path_to_working, path_to_repository);
        let path_to_file = atomic.queue_create(path_to_snapshot).map_err(|err| err.add_generic_message("During a snapshot operation"))?;
        // Queue the snapshot to be written to disk
        Self::write_simple_snapshot(&snapshot, path_to_file.as_path(), path_to_working)?;
        // Process all the files that will be stored in this snapshot and prepare to store them into the repository
        // TODO: Need to include the parent here to know how to process the files
        // TODO: store_snapshot should return the state of the files being stored
        // TODO: Pass in the list of files to be stored rather than the snapshot
        file_storage.store_snapshot(&snapshot, &mut atomic)
            .map_err(|err| err.add_generic_message("Store snapshot failed during a child snapshot"))?;
        // The parent needs to be modified so that it knows about its new child
        // TODO: Do we need to check for cyclic references - it should be impossible since we create the child, moving a snapshot would require checking for cyclic references
        let parent_snapshot = self.snapshot_by_hash_mut(&parent_hash, path_to_repository, path_to_working)
            .map_err(|err| err
                .add_generic_message("While trying to get the snapshot that is gaining a child snapshot"))?;
        // Adjust the snapshot record
        parent_snapshot.add_child(snapshot.get_hash().clone());
        // Queue the file replacement operation
        let path_to_parent = path_to_repository.join(RepositorySnapshots::get_path()).join(parent_snapshot.get_hash().to_string());
        let path_to_parent_snapshot = atomic.queue_replace(path_to_parent.as_path())
            .map_err(|err| err.add_generic_message("While trying to replace a snapshot that gained a child"))?;
        // Rewrite the snapshot
        // TODO: Get the parent location as it might not be simple
        Self::write_simple_snapshot(parent_snapshot, path_to_parent_snapshot.as_path(), path_to_working)
            .map_err(|err| err.add_generic_message("While adding a child snapshot"))?;
        let is_new_parent = parent_snapshot.get_children().len() == 1;
        // Update state and queue file changes
        self.update_state(&mut atomic, &snapshot, is_new_parent)?;
        // Process all queued file changes
        self.snapshots.push(snapshot);
        if let Err(error) = atomic.complete() {
            unimplemented!("Failed to complete atomic while adding a child snapshot, {}", error);
        }
        let snapshot_hash = self.snapshots[self.snapshots.len() - 1].get_hash().clone(); // Index is reduced by one in both cases since the hash was already added to the index
        let id = SnapshotId::Indexed(self.snapshots.len() - 1, snapshot_hash);
        Ok(id)
    }
}