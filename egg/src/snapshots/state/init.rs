use super::{SnapshotsState, StateBuilder};
use crate::error::{Error, UnderlyingError};
use crate::storage::stream::{ReadEggExt, WriteEggExt};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use std::path;
use std::fs;
use std::io;
use std::collections::VecDeque;
use std::convert::TryFrom;

// Public interfaces
impl SnapshotsState {
    const VERSION: u16 = 1;
    pub const STATE_FILE_NAME: &'static str = "state";
    pub const SNAPSHOTS_PATH: &'static str = "snapshots";
    //SnapshotsState::STATE_PATH).join(SnapshotsState::STATE_FILE_NAME))

    /// Creates a new snapshot storage state file if needed and returns the current state of snapshots in the repository
    // TODO: The problem with this method is that we have no idea if a snapshot state file was expected
    pub fn load(path_to_repository: &path::Path) -> Result<SnapshotsState, Error> {
        // TODO: Create a snapshots directory to place the files in
        // TODO: This probably needs to be atomic to recover from a bad create repository
        // Create the snapshot storage state file
        let path_to_state = path_to_repository.join(Self::SNAPSHOTS_PATH).join(Self::STATE_FILE_NAME);
        // Since we are initialising we expect there to already be a snapshots state file
        if path_to_state.exists() == false {
            return Err(Error::invalid_repository().add_generic_message("No snapshots state file was found"));
        }
        SnapshotsState::parse_state_file(path_to_state)
    }
    
    // Creates a new state file with presumed defaults, ie empty state
    pub fn new(path_to_repository: &path::Path) -> Result<SnapshotsState, Error> {
        // TODO: Create a snapshots directory to place the files in
        // TODO: This probably needs to be atomic to recover from a bad create repository
        let path_to_state = path_to_repository.join(Self::SNAPSHOTS_PATH).join(Self::STATE_FILE_NAME);
        // Create the snapshot storage state file
        let snapshot_file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path_to_state.as_path())
                .map_err(|err| Error::file_error(Some(UnderlyingError::from(err)))
                    .add_user_message("Failed to create a configuration file while creating a new repository")
                    .add_debug_message(format!("Failed to create a snapshot state file, the path was {}", path_to_state.display())))?;
        
        let file_writer = io::BufWriter::new(snapshot_file);
        let initial_state = SnapshotsState {
            working_snapshot: None,
            latest_snapshot: None,
            recent_snapshots: VecDeque::new(),
            root_snapshots: Vec::new(),
            end_snapshots: Vec::new(),
            path_to_state_file: path_to_state,
        };
        // Write initial state
        SnapshotsState::write_state_file(&initial_state, file_writer)
            .map_err(|err|err.add_generic_message("While creating a new snapshots state file"))?;
        Ok(initial_state)
    }

    /// Loads the current state of snapshots
    pub(in crate::snapshots::state) fn parse_state_file(path_to_state: path::PathBuf) -> Result<SnapshotsState, Error> {
        let snapshot_file = fs::OpenOptions::new()
            .read(true)
            .open(path_to_state.as_path())
            .map_err(|err| Error::file_error(Some(UnderlyingError::from(err)))
                .add_debug_message(format!("While trying to open the snapshot state file an error occured, the path used was {}", path_to_state.display()))
                .add_user_message("Failed to open a snapshot configuration file"))?;
        let mut file_reader = io::BufReader::new(snapshot_file);
        // This version is the most recent so parse it
        // Read the version of the snapshot state file
        let version = file_reader.read_u16::<LittleEndian>()
            .map_err(|err| Error::file_error(Some(UnderlyingError::from(err)))
                .add_generic_message("Failed to read the version of the snapshot state file"))?;
        if version != SnapshotsState::VERSION {
            // TODO: Upgrade path for snapshot state file
            unimplemented!(
                "Reached the upgrade path for snapshot state, version recorded was: {:?}",
                version
            );
        }
        let mut snapshot_state = SnapshotsState {
            end_snapshots: Vec::new(),
            root_snapshots: Vec::new(),
            recent_snapshots: VecDeque::new(),
            latest_snapshot: None,
            working_snapshot: None,
            path_to_state_file: path_to_state,
        };
        let mut builder = StateBuilder::new(&mut snapshot_state);
        // Read working snapshot
        let working_snapshot = file_reader.read_optional_hash()
            .map_err(|err| err
                .add_generic_message("Failed to read working snapshot"))?;
        builder.set_working_snapshot(working_snapshot);
        // Read latest snapshot
        let latest_snapshot = file_reader.read_optional_hash()
            .map_err(|err| err
                .add_generic_message("Failed to read latest snapshot"))?;
        builder.set_latest_snapshot(latest_snapshot);
        // Read number of recent snapshot files
        let recent_snapshots_total = file_reader.read_u16::<LittleEndian>()
            .map_err(|err| Error::file_error(Some(UnderlyingError::from(err)))
                .add_generic_message("Failed to read total number of snapshots"))?;
        // Fill the Dequeue with the most recent snapshots
        // let mut recent_snapshots = VecDeque::with_capacity(usize::from(recent_snapshots_total));
        for _ in 0..recent_snapshots_total {
            let recent_snapshot = file_reader.read_hash()
                .map_err(|err| err
                    .add_generic_message("While reading a recent snapshot loading snapshots state"))?;
            builder.add_recent_snapshot(recent_snapshot);
        }
        // Read number of root snapshots
        let total_root_ids = file_reader.read_u16::<LittleEndian>()
            .map_err(|err| Error::file_error(Some(UnderlyingError::from(err)))
                .add_generic_message("Failed to read total root snapshots"))?;
        // Read Root ID's
        // let mut root_snapshots = Vec::with_capacity(usize::from(total_root_ids));
        for _ in 0..total_root_ids {
            let root_id = file_reader.read_hash().map_err(|err| err.add_generic_message("Failed to read root snapshot hash"))?;
            builder.add_root_node(root_id);
        }
        // Read number of end snapshots
        let total_end_ids = file_reader.read_u16::<LittleEndian>()
            .map_err(|err| Error::file_error(Some(UnderlyingError::from(err)))
                .add_generic_message("Failed to read number of snapshots"))?;
        // Read end ID's
        // let mut end_snapshots = Vec::with_capacity(usize::from(total_end_ids));
        for _ in 0..total_end_ids {
            let end_id = file_reader.read_hash()
                .map_err(|err| err  
                    .add_generic_message("Failed to read end snapshot hash"))?;
            builder.add_end_node(end_id);
        }
        // We store a copy of the path to the state file since we require it when making changes to state
        
        Ok(snapshot_state)
    }
    #[allow(dead_code)]
    fn check_state() -> () {
        //TODO: Used to validate a state file after recovering from a interrupted operation
    }
    
    // Writes the current state of the snapshot storage system
    pub(in crate::snapshots::state) fn write_state_file(
        current_state: &SnapshotsState,
        mut file_writer: io::BufWriter<fs::File>,
    ) -> Result<(), Error> {
        // TODO: Use a generic writer instead of a BufWriter
        // Write the version of the snapshot state file
        file_writer.write_u16::<LittleEndian>(SnapshotsState::VERSION)
            .map_err(|err| Error::file_error(Some(UnderlyingError::from(err)))
                .add_generic_message("While writing the version of the snapshot state file"))?;
        // Write the ID of the working snapshot - This hash may not be present even after initialization
        file_writer.write_optional_hash(current_state.working_snapshot.as_ref())
            .map_err(|err| err
                .add_generic_message("While writing the working snapshot hash"))?;
        // Write the snapshot ID of the latest snapshot
        file_writer.write_optional_hash(current_state.latest_snapshot.as_ref())
            .map_err(|err| err
                .add_generic_message("While updating the latest snapshot in the state file"))?;
        // Write number of recently accessed data files
        // TODO: This should be a u8 and only store the most recent 10?
        file_writer.write_u16::<LittleEndian>(u16::try_from(current_state.recent_snapshots.len()).unwrap())
            .map_err(|err| Error::file_error(Some(UnderlyingError::from(err)))
                .add_generic_message("Failed to write the number of recent snapshots"))?;
        
        // Write the recently used hashes
        for recent_hash in &current_state.recent_snapshots {
            file_writer.write_hash(recent_hash)
                .map_err(|err| err
                    .add_generic_message("Failed to write recent hash in state file"))?;
        }
        // Write number of root snapshots
        file_writer.write_u16::<LittleEndian>(u16::try_from(current_state.root_snapshots.len()).unwrap())
            .map_err(|err| Error::file_error(Some(UnderlyingError::from(err)))
                .add_generic_message("Failed to write number of root snapshots"))?;
        for root_id in &current_state.root_snapshots {
            file_writer.write_hash(&root_id)
                .map_err(|err| err
                    .add_generic_message("Failed to write root snapshot in state file"))?;
        }
        // Write number of end snapshots
        file_writer.write_u16::<LittleEndian>(u16::try_from(current_state.end_snapshots.len()).unwrap())
            .map_err(|err| Error::file_error(Some(UnderlyingError::from(err)))
                .add_generic_message("Failed to write number of end snapshots"))?;
        // Write the end snapshots
        for end_id in &current_state.end_snapshots {
            file_writer.write_hash(&end_id)
                .map_err(|err| err
                    .add_generic_message("Failed to write end snapshot in state file"))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::hash::Hash;
    use crate::snapshots::SnapshotsState;
    use std::collections::VecDeque;
    use std::io;
    use testspace::TestSpace;
    #[test]
    fn test_read_write_state() {
        let root = Hash::generate_random_hash();
        let mut root_hashes = Vec::new();
        root_hashes.push(root);
        let working = Hash::generate_random_hash();

        
        let ts = TestSpace::new();
        let tsf = ts.create_tsf();
        let test_state = SnapshotsState {
            latest_snapshot: None,
            working_snapshot: Some(working),
            recent_snapshots: VecDeque::new(),
            root_snapshots: root_hashes,
            end_snapshots: Vec::new(),
            path_to_state_file: tsf.get_path().to_path_buf(),
        };
        {
            let file = tsf.open_file();
            let file_writer = io::BufWriter::new(file);
            SnapshotsState::write_state_file(&test_state, file_writer).expect("Failed to write state");
        }
        let path_to_state = tsf.get_path();
        let result_state = SnapshotsState::parse_state_file(path_to_state.to_path_buf()).expect("Failed to read state");
        assert_eq!(result_state, test_state);
    }
    
}