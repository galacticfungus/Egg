use super::{SnapshotsState, StateBuilder};
use crate::atomic::AtomicUpdate;
use crate::error::{Error, UnderlyingError};
use crate::hash::Hash;

use std::path;
use std::{fs, io};

impl SnapshotsState {
    // Allows a caller to change the current state of snapshots, ie changing working snapshot etc
    pub fn change_state<F>(
        &mut self,
        atomic_updater: &mut AtomicUpdate,
        callback: F,
    ) -> Result<(), Error>
    where
        F: FnOnce(&mut StateBuilder) -> Result<(), Error>,
    {
        let mut builder = StateBuilder::new(self);
        // Get the requried changes from the caller
        callback(&mut builder).map_err(|err| {
            err.add_generic_message("Change state callback method has returned a user error")
        })?;
        // Queue replacing the current state file with the new one
        let path_to_file = atomic_updater
            .queue_replace(self.path_to_state_file.as_path())
            .map_err(|err| err.add_generic_message("During a change snapshot state operation"))?;
        // Prepares the new state on disk, this wont actually replace the old state until the atomic operation completes
        SnapshotsState::update_state(path_to_file.as_path(), &self).map_err(|err| {
            err.add_generic_message("While trying to change the current snapshot state")
        })?;
        Ok(())
    }

    /// Writes the new state to disk, since this uses the atomic updater, the changes will not be written immediately
    fn update_state(
        path_to_state: &path::Path,
        state_to_write: &SnapshotsState,
    ) -> Result<(), Error> {
        let state_file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path_to_state)
            .map_err(|err| {
                Error::file_error(Some(UnderlyingError::from(err)))
                    .add_generic_message("Failed to open state file while trying to update it")
            })?;
        let file_writer = io::BufWriter::new(state_file);
        // Write the new state
        SnapshotsState::write_state_file(state_to_write, file_writer)
            .map_err(|err| err.add_generic_message("While updating the snapshot state"))?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn get_recent_snapshots(&self) -> impl Iterator<Item = &Hash> {
        self.recent_snapshots.iter()
    }

    #[allow(dead_code)]
    pub fn get_root_snapshots(&self) -> &[Hash] {
        self.root_snapshots.as_slice()
    }

    #[allow(dead_code)]
    pub fn get_end_snapshots(&self) -> &[Hash] {
        self.end_snapshots.as_slice()
    }

    // Get the index of the current snapshot
    pub fn get_working_snapshot(&self) -> Option<Hash> {
        self.working_snapshot.clone()
    }

    // Get the path to the most recent snapshot data file
    pub fn get_latest_snapshot(&self) -> Option<Hash> {
        self.latest_snapshot.clone()
    }
}
