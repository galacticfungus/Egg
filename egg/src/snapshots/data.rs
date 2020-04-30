use super::{RepositorySnapshots, Snapshot, SnapshotId};
use crate::{atomic::AtomicUpdate, error::Error};

use std::path;

impl RepositorySnapshots {
    // Updates the snapshot state when taking a snapshot
    pub fn update_state(&mut self, atomic: &mut AtomicUpdate, snapshot: &Snapshot, is_new_parent: bool) -> Result<(), Error> {
        self.state.change_state(atomic, |state_data| {
            if let Some(parent) = snapshot.get_parent() {
                // Remove the parent as a end node if it is a new parent, as long as we don't insert snapshots between snapshots this will work
                if is_new_parent {
                    state_data.remove_end_node(parent);
                }
                
            } else {
                // If this snapshot has no parent add it as a root snapshot
                state_data.add_root_node(snapshot.get_hash().clone());
                state_data.add_end_node(snapshot.get_hash().clone());
            }
            // Regardless of whether the snapshot is a child or not it will always be a end node as we never insert between nodes
            state_data.add_end_node(snapshot.get_hash().clone());
            // Update latest snapshot
            state_data.set_latest_snapshot(Some(snapshot.get_hash().clone()));
            Ok(())
        })
    }

    pub fn get_latest_snapshot(&self, path_to_repository: &path::Path) -> Result<Option<SnapshotId>, Error> {
        if let Some(hash) = self.state.get_latest_snapshot() {
            let id = self.get_id(&hash, path_to_repository)?;
            return Ok(Some(id.clone()));
        }
        Ok(None)
    }

    pub fn get_working_snapshot(&self, path_to_repository: &path::Path) -> Result<Option<SnapshotId>, Error> {
        if let Some(hash) = self.state.get_working_snapshot() {
            let id = self.get_id(&hash, path_to_repository)?;
            return Ok(Some(id.clone()));
        }
        Ok(None)
    }

    // Returns all the snapshots that have no parent - meaning that they start a chain of snapshots (ie start nodes)
    pub fn get_root_snapshots(&self, path_to_repository: &path::Path) -> Result<Vec<SnapshotId>, Error> {
        let mut root_ids = Vec::new();
        for root_hash in self.state.get_root_snapshots() {
            let id = self.get_id(root_hash, path_to_repository)?;
            root_ids.push(id);
        }
        Ok(root_ids)
    }

    /// Returns all the snapshots that have no children - meaning that they are the end of a list of snapshots (ie end nodes)
    pub fn get_end_snapshots(&self, path_to_repository: &path::Path) -> Result<Vec<SnapshotId>, Error> {
        let mut end_ids = Vec::new();
        for end_hash in self.state.get_end_snapshots() {
            let id = self.get_id(end_hash, path_to_repository)?;
            end_ids.push(id);
        }
        Ok(end_ids)
    }
}