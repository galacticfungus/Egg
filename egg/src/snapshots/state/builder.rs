use super::{SnapshotsState, StateBuilder};
use crate::hash::Hash;

impl<'a> StateBuilder<'a> {
    pub fn new(state_to_modify: &'a mut SnapshotsState) -> StateBuilder {
        StateBuilder {
            state: state_to_modify,
        }
    }

    pub fn set_latest_snapshot(&mut self, latest: Option<Hash>) {
        self.state.latest_snapshot = latest;
    }

    pub fn set_working_snapshot(&mut self, working: Option<Hash>) {
        self.state.working_snapshot = working;
    }

    // Adds a hash to the recent snapshots, only the most recent 10 snapshots are stored
    pub fn add_recent_snapshot(&mut self, recent_hash: Hash) {
        if self.state.recent_snapshots.len() > 10 {
            self.state.recent_snapshots.pop_front();
            self.state.recent_snapshots.push_back(recent_hash);
        } else {
            self.state.recent_snapshots.push_back(recent_hash);
        }
    }

    pub fn add_root_node(&mut self, root_hash: Hash) {
        self.state.root_snapshots.push(root_hash);
    }

    pub fn remove_root_node(&mut self, root_hash: &Hash) {
        if let Some(index) = self
            .state
            .root_snapshots
            .iter()
            .position(|hash| hash == root_hash)
        {
            self.state.root_snapshots.swap_remove(index);
        } else {
            // TODO: This needs to be handled better
            panic!("Attempted to remove root hash that does not exist");
        }
    }

    pub fn remove_end_node(&mut self, end_hash: &Hash) {
        if let Some(index) = self
            .state
            .end_snapshots
            .iter()
            .position(|hash| hash == end_hash)
        {
            self.state.end_snapshots.swap_remove(index);
        } else {
            // TODO: This needs to be handled better
            panic!("Attempted to remove end hash that does not exist");
        }
    }

    pub fn add_end_node(&mut self, end_hash: Hash) {
        self.state.end_snapshots.push(end_hash);
    }
}
