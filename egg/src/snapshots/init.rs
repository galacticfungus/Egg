use super::{RepositorySnapshots, SnapshotsState};
use crate::{atomic::AtomicUpdate, error::{Error, UnderlyingError}};

use std::path;
use std::collections::HashMap;

impl RepositorySnapshots {
    pub const SNAPSHOTS_PATH: &'static str = "snapshots";
    /// Initializes the snapshots system based on the current repository
    pub(crate) fn new(path_to_repository: &path::Path, path_to_working: &path::Path) -> Result<RepositorySnapshots, Error> {
        // TODO: This needs to change so that we always initialize which either loads in state or creates new state
        // initialize state - this requires a path to an initial snapshot data file
        // TODO: Check that a snapshot state
        let snapshot_directory = path_to_repository.join(Self::SNAPSHOTS_PATH);
        if snapshot_directory.exists() == false {
            std::fs::create_dir(snapshot_directory)
                .map_err(|err| Error::file_error(Some(UnderlyingError::from(err)))
                    .add_generic_message("Failed to create snapshots directory"))?;
        }
        let state = SnapshotsState::new(path_to_repository)?;
        // TODO: Need to obtain a rough estimate of the initial size of the memory structures
        let snapshot_storage = RepositorySnapshots {
            state,
            snapshots: Vec::new(), // TODO: Get estimate of needed capacity before creating
            index: HashMap::new(), // This is all the initialization the index needs currently
        };
        Ok(snapshot_storage)
    }

    /// This takes the current stored state and processes it
    pub(crate) fn load(path_to_working: &path::Path, path_to_repository: &path::Path) -> Result<RepositorySnapshots, Error> {
        // TODO: Check that the snapshot system was left in a valid state
        // TODO: Restore needs to focus on restoring based on the current state and needs to be a method
        // Restore State
        let state = SnapshotsState::load(path_to_repository)?;
        // Restore Data
        // TODO: Need to init seperately since we or may not init with working snapshot
        let snapshots = Vec::new();

        // let RepositorySnapshots {state, snapshots, index} = &mut storage;
        let mut storage = RepositorySnapshots {
            state,
            snapshots,
            index: HashMap::new(), // This is all that is done to restore the index currently
        };
        {
        // let RepositorySnapshots {state, snapshots, index} = &mut storage;
        if let Some(working_snapshot) = storage.state.get_working_snapshot() {
            let location = storage.locate_hash(&working_snapshot, path_to_repository)
                .ok_or(Error::invalid_state(None)
                    .add_generic_message("The current working snapshot could not be located"))?;
            storage.load_snapshot(&working_snapshot, location, path_to_repository, path_to_working)?;
        }
        }
        // Load the latest snapshot
        if let Some(latest_snapshot) = storage.state.get_latest_snapshot().clone() {
            if storage.is_indexed(&latest_snapshot) == false {
                let location = storage.locate_hash(&latest_snapshot, path_to_repository)
                    .ok_or(Error::invalid_state(None)
                        .add_generic_message("The current working snapshot could not be located"))?;
                storage.load_snapshot(&latest_snapshot, location, path_to_repository, path_to_working)?;
            }
        }
        // Load recently used snapshots
        // TODO: How useful is this in reality - it means the last 10 used snapshots are pre loaded
        
        for index_of_recent in 0..storage.state.recent_snapshots.len() {
            // Only load this snapshot if it isn't already loaded
            let recent_snapshot = storage.state.recent_snapshots[index_of_recent].clone();
            if storage.is_indexed(&recent_snapshot) == false {
                let location = storage.locate_hash(&recent_snapshot, path_to_repository)
                    .ok_or(Error::invalid_state(None)
                        .add_generic_message("The current working snapshot could not be located"))?;
                storage.load_snapshot(&recent_snapshot, location, path_to_repository, path_to_working)?;
            }
        }
        // TODO: If the number of snapshots loaded is below a certain threshold then should we load more based on parents or children of snapshots loaded
        // NOTE: We probably need to base this on parameters passed to when the snapshot system is loaded, basically preloading should be based on client needs
        Ok(storage)
    }
}