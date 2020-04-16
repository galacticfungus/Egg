use crate::error::Error;
use crate::hash::Hash;
use crate::AtomicUpdate;
use crate::storage::stream::{ReadEggExt, WriteEggExt};
use byteorder::{self, LittleEndian, ReadBytesExt, WriteBytesExt};
use std::collections::VecDeque;
use std::convert::TryFrom;
use std::fs;
use std::io::{self};
use std::iter::Iterator;
use std::path;

type Result<T> = std::result::Result<T, Error>;

/// Represents the state of the snapshot system, ie what is the latest snapshot, what are the root snapshots
// TODO: Move all the state fileIO stuff into storage, state can be all different states tracked
// TODO: impl Storable for storage state
#[derive(Debug)]
pub struct StorageState {
    working_snapshot: Option<Hash>,   // Hash ID of the current snapshot
    latest_snapshot: Option<Hash>,    // Path to the recent snapshot file
    recent_snapshots: VecDeque<Hash>, // A list of recently accessed snapshots
    root_snapshots: Vec<Hash>, // The root snapshots in the repository, ie snapshots with no parent
                               // TODO: Add a usize to track the number of snapshots current stored in current_snapshot_file
                               // TODO: A separate file that tracks only global snapshot changes
    end_snapshots: Vec<Hash>,
}

// Writing Data
impl StorageState {
    // Writes the current state of the snapshot storage system
    fn write_state(
        current_state: &StorageState,
        mut file_writer: io::BufWriter<fs::File>,
    ) -> Result<()> {
        // TODO: Use a generic writer instead of a BufWriter
        // Write the version of the snapshot file
        if let Err(error) = file_writer.write_u16::<LittleEndian>(StorageState::VERSION) {
            unimplemented!();
        };
        // Write the ID of the working snapshot - This hash may not be present even after initialization
        if let Err(error) = file_writer.write_optional_hash(current_state.working_snapshot.as_ref())
        {
            unimplemented!();
        };
        // Write the snapshot ID of the latest snapshot
        if let Err(error) = file_writer.write_optional_hash(current_state.latest_snapshot.as_ref())
        {
            unimplemented!();
        };
        // Write number of recently accessed data files
        // TODO: This should be a u8 and only store the most recent 10?
        if let Err(error) = file_writer.write_u16::<LittleEndian>(u16::try_from(current_state.recent_snapshots.len()).unwrap())
        {
            unimplemented!();
        }
        // Write the recently accessed hashes
        for recent_hash in &current_state.recent_snapshots {
            if let Err(error) = file_writer.write_hash(recent_hash) {
                unimplemented!();
            }
        }
        // Write number of root snapshots
        if let Err(error) = file_writer.write_u16::<LittleEndian>(u16::try_from(current_state.root_snapshots.len()).unwrap())
        {
            unimplemented!();
        }
        for root_id in &current_state.root_snapshots {
            if let Err(error) = file_writer.write_hash(&root_id) {
                unimplemented!();
            }
        }
        // Write number of end snapshots
        if let Err(error) = file_writer.write_u16::<LittleEndian>(u16::try_from(current_state.end_snapshots.len()).unwrap())
        {
            unimplemented!();
        }
        for end_id in &current_state.end_snapshots {
            if let Err(error) = file_writer.write_hash(&end_id) {
                unimplemented!();
            }
        }
        Ok(())
    }
}

// Reading Data
impl StorageState {
    const VERSION: u16 = 1;
    const STATE_FILE_NAME: &'static str = "snapshots/state";

    fn check_state() -> () {
        //TODO: Used to validate a state file after recovering from a interrupted operation
    }

    // Read the most current version of the state file and returns the current state
    fn read_state(mut file_reader: io::BufReader<fs::File>) -> Result<StorageState> {
        // Read the version of the snapshot state file
        let version = match file_reader.read_u16::<LittleEndian>() {
            Ok(version) => version,
            Err(error) => unimplemented!(),
        };
        if version != StorageState::VERSION {
            // TODO: Upgrade path for snapshot state file
            unimplemented!(
                "Reached the upgrade path for snapshot state, version recorded was: {:?}",
                version
            );
        }
        // Read working snapshot
        let working_snapshot = match file_reader.read_optional_hash() {
            Ok(current_hash) => current_hash,
            Err(error) => unimplemented!(),
        };
        // Read latest snapshot
        let latest_snapshot = match file_reader.read_optional_hash() {
            Ok(current_hash) => current_hash,
            Err(error) => unimplemented!(),
        };
        // Read number of recent snapshot files
        let recent_snapshots_total = match file_reader.read_u16::<LittleEndian>() {
            Ok(total_files) => total_files,
            Err(error) => unimplemented!(),
        };
        // TODO: Iterate mutably over the vector and fill it
        let mut recent_snapshots = VecDeque::with_capacity(usize::from(recent_snapshots_total));
        for _ in 0..recent_snapshots_total {
            let file_path = match file_reader.read_hash() {
                Ok(file_path) => file_path,
                Err(error) => unimplemented!(),
            };
            recent_snapshots.push_back(file_path);
        }
        // Read number of root snapshots
        let total_root_ids = match file_reader.read_u16::<LittleEndian>() {
            Ok(total_root_ids) => total_root_ids,
            Err(error) => unimplemented!("Failed to read root ids, no error handling for snapshot state"),
        };
        // Read Root ID's
        let mut root_snapshots = Vec::with_capacity(usize::from(total_root_ids));
        for _ in 0..total_root_ids {
            let root_id = match file_reader.read_hash() {
                Ok(root_id) => root_id,
                Err(error) => unimplemented!(),
            };
            root_snapshots.push(root_id);
        }
        // Read number of end snapshots
        let total_end_ids = match file_reader.read_u16::<LittleEndian>() {
            Ok(total_end_ids) => total_end_ids,
            Err(error) => unimplemented!("Failed to read root ids, no error handling for snapshot state"),
        };
        // Read end ID's
        let mut end_snapshots = Vec::with_capacity(usize::from(total_end_ids));
        for _ in 0..total_end_ids {
            let end_id = match file_reader.read_hash() {
                Ok(end_id) => end_id,
                Err(error) => unimplemented!(),
            };
            end_snapshots.push(end_id);
        }
        let state = StorageState {
            working_snapshot,
            latest_snapshot,
            recent_snapshots,
            root_snapshots,
            end_snapshots,
        };
        Ok(state)
    }

    /// Loads the current state of the repositories snapshot storage
    pub fn load_state(root_path: &path::Path) -> Result<StorageState> {
        let path_to_state = root_path.join(StorageState::STATE_FILE_NAME);
        let snapshot_file = match fs::OpenOptions::new().read(true).open(path_to_state.as_path())
        {
            Ok(snapshot_file) => snapshot_file,
            Err(error) => unimplemented!("Failed to read the snapshot state file and error handling isn't implemented"),
        };
        let file_reader = io::BufReader::new(snapshot_file);
        // This version is the most recent so parse it
        let current_state = StorageState::read_state(file_reader)?;
        return Ok(current_state);
    }
}

// Public interfaces
impl StorageState {
    /// Creates a new snapshot storage state file in the path given, the validity of the target path is not checked
    pub fn initialize(root_path: &path::Path) -> Result<StorageState> {
        // TODO: Create a snapshots directory to place the files in
        // TODO: This probably needs to be atomic to recover from a bad create repository
        // Create the snapshot storage state file
        let path_to_snapshot = root_path.join(StorageState::STATE_FILE_NAME);
        let snapshot_file = match fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path_to_snapshot.as_path())
        {
            Ok(snapshot_file) => snapshot_file,
            Err(error) => unimplemented!(),
        };
        let file_writer = io::BufWriter::new(snapshot_file);
        let initial_state = StorageState {
            working_snapshot: None,
            latest_snapshot: None,
            recent_snapshots: VecDeque::new(),
            root_snapshots: Vec::new(),
            end_snapshots: Vec::new(),
        };
        // Write initial state
        if let Err(error) = StorageState::write_state(&initial_state, file_writer) {
            unimplemented!("Failed to write the snapshot state during initialization");
        }
        Ok(initial_state)
    }

    // Get the index of the current snapshot
    pub fn get_working_snapshot(&self) -> Option<&Hash> {
        self.working_snapshot.as_ref()
    }

    // Get the path to the most recent snapshot data file
    pub fn get_latest_snapshot(&self) -> Option<&Hash> {
        self.latest_snapshot.as_ref()
    }
}

impl StorageState {
    pub fn change_state<F>(&mut self, atomic_updater: &mut AtomicUpdate, callback: F) -> Result<()>
        where F: FnOnce(&mut StorageStateBuilder) -> Result<()>,
    {
        println!("Start internal state change");
        let new_state = StorageState {
            latest_snapshot: self.latest_snapshot.clone(),
            working_snapshot: self.working_snapshot.clone(),
            recent_snapshots: self.recent_snapshots.clone(),
            root_snapshots: self.root_snapshots.clone(),
            end_snapshots: self.end_snapshots.clone(),
        };
        println!("Making builder");
        let mut builder = StorageStateBuilder::new(new_state);
        if let Err(error) = callback(&mut builder) {
            unimplemented!();
        }
        let new_state = builder.build();
        // Write the state to disk
        println!("Writing state builder created");
        let path_to_file = atomic_updater.queue_replace(StorageState::STATE_FILE_NAME).map_err(|err| err.add_generic_message("During a change snapshot state operation"))?;
        if let Err(error) = StorageState::update_state(path_to_file.as_path(), &new_state) {
            // Since this only writes to a temporary file allocated by atomic then state is always safe until the operation completes
            unimplemented!("No error handling when update_state fails to record the new state in a temporary file");
        }
        // Replace self with the new state
        // This state wont be represented on disk until the current operation finishes and the atomic update executes and completes
        std::mem::replace(self, new_state);
        Ok(())
    }

    /// Writes the new state to disk, since this uses the atomic updater, the changes will not be written immediately
    fn update_state(path_to_state: &path::Path, state_to_write: &StorageState) -> Result<()> {
        let state_file = match fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path_to_state)
        {
            Ok(snapshot_file) => snapshot_file,
            Err(error) => {
                unimplemented!("Failed to open new state file {}", path_to_state.display())
            }
        };
        let file_writer = io::BufWriter::new(state_file);
        // Write the new state
        if let Err(error) = StorageState::write_state(state_to_write, file_writer) {
            unimplemented!();
        }
        Ok(())
    }

    pub fn get_recent_snapshots(&self) -> impl Iterator<Item = &Hash> {
        self.recent_snapshots.iter()
    }

    pub fn get_root_snapshots(&self) -> &[Hash] {
        self.root_snapshots.as_slice()
    }

    pub fn get_end_snapshots(&self) -> &[Hash] {
        self.end_snapshots.as_slice()
    }
}

pub struct StorageStateBuilder {
    state: StorageState,
}

impl StorageStateBuilder {
    pub fn new(state: StorageState) -> StorageStateBuilder {
        StorageStateBuilder { state }
    }

    pub fn build(self) -> StorageState {
        self.state
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
        if let Some(index) = self.state.root_snapshots.iter().position(|hash| hash == root_hash)
        {
            self.state.root_snapshots.swap_remove(index);
        } else {
            // TODO: This needs to be handled better
            panic!("Attempted to remove root hash that does not exist");
        }
    }

    pub fn remove_end_node(&mut self, end_hash: &Hash)  {
        if let Some(index) = self.state.end_snapshots.iter().position(|hash| hash == end_hash)
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

#[cfg(test)]
mod tests {
    use crate::hash::Hash;
    use crate::AtomicUpdate;
    use crate::snapshots::StorageState;
    use std::collections::VecDeque;
    use std::io::{self, Seek};
    use testspace::TestSpace;

    impl PartialEq for StorageState {
        fn eq(&self, other: &Self) -> bool {
            if self.working_snapshot != other.working_snapshot {
                return false;
            }
            if self.latest_snapshot != other.latest_snapshot {
                return false;
            }
            return true;
        }
    }

    #[test]
    fn test_state_init() {
        // Tests that the snapshot state file is correctly initialized and loaded
        let ts = TestSpace::new();
        let base_path = ts.get_path();
        // Since state never reads the snapshot data file using a fake path is fine
        let initial =
            StorageState::initialize(base_path).expect("Failed to initialize the repository");
        let loaded = StorageState::load_state(base_path)
            .expect("Failed to load the repository that was just initialized");
        assert_eq!(initial, loaded);
    }

    #[test]
    fn test_read_write_state() {
        let root = Hash::generate_random_hash();
        let mut root_hashes = Vec::new();
        root_hashes.push(root);
        let working = Hash::generate_random_hash();

        let test_state = StorageState {
            latest_snapshot: None,
            working_snapshot: Some(working),
            recent_snapshots: VecDeque::new(),
            root_snapshots: root_hashes,
            end_snapshots: Vec::new(),
        };
        let ts = TestSpace::new();
        let tsf = ts.create_tsf();

        {
            let file = tsf.open_file();
            let file_writer = io::BufWriter::new(file);
            StorageState::write_state(&test_state, file_writer).expect("Failed to write state");
        }
        let file = tsf.open_file();
        let mut file_reader = io::BufReader::new(file);
        file_reader
            .seek(io::SeekFrom::Start(0))
            .expect("Failed to seek");
        let result_state = StorageState::read_state(file_reader).expect("Failed to read state");
        assert_eq!(result_state, test_state);
    }

    #[test]
    fn test_get_set_latest_hash() {
        let known_hash: [u8; 64] = [
            0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11,
            0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF,
            0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11,
            0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF,
            0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF,
        ];
        let latest_hash = Hash::from(known_hash.as_ref());
        let ts = TestSpace::new();
        let ts2 = ts.create_child();
        let working_path = ts.get_path();
        let repository_path = ts2.get_path();
        let known_hash = latest_hash.clone();
        {
            let mut au =
                AtomicUpdate::init(repository_path, working_path).expect("Failed to initialize atomic updater");
            let mut initial =
                StorageState::initialize(repository_path).expect("Failed to initialize state");
            initial
                .change_state(&mut au, |state| {
                    state.set_latest_snapshot(Some(latest_hash));
                    Ok(())
                })
                .expect("Failed to change state");
            au.complete().expect("Failed to update files atomically");
        }

        let result = StorageState::load_state(repository_path).expect("Failed to load state");
        let result_hash = result.get_latest_snapshot().unwrap();
        assert_eq!(result_hash, &known_hash);
    }

    #[test]
    fn test_get_set_working_hash() {
        let known_hash: [u8; 64] = [
            0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11,
            0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF,
            0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11,
            0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF,
            0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF,
        ];
        let latest_hash = Hash::from(known_hash.as_ref());
        let ts = TestSpace::new();
        let ts2 = ts.create_child();
        let working_path = ts.get_path();
        let repository_path = ts2.get_path();
        let known_hash = latest_hash.clone();
        {
            let mut initial =
                StorageState::initialize(repository_path).expect("Failed to initialize state");
            let mut au =
                AtomicUpdate::init(repository_path, working_path).expect("Failed to initialize atomic updater");
            initial
                .change_state(&mut au, |state| {
                    state.set_working_snapshot(Some(latest_hash));
                    Ok(())
                })
                .expect("Failed to change state");
            au.complete().expect("Failed to update files atomically");
        }
        let result = StorageState::load_state(repository_path).expect("Failed to load state");
        let result_hash = result.get_working_snapshot().unwrap();
        assert_eq!(result_hash, &known_hash);
    }

    #[test]
    fn test_add_remove_root_snapshots() {
        let known_hash: [u8; 64] = [
            0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11,
            0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF,
            0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11,
            0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF,
            0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF,
        ];
        let latest_hash = Hash::from(known_hash.as_ref());
        let ts = TestSpace::new();
        let ts2 = ts.create_child();
        let working_path = ts.get_path();
        let repository_path = ts2.get_path();
        let known_hash = latest_hash.clone();
        {
            let mut initial =
                StorageState::initialize(repository_path).expect("Failed to initialize state");
            let mut au =
                AtomicUpdate::init(repository_path, working_path).expect("Failed to initialize atomic updater");
            initial
                .change_state(&mut au, |state| {
                    state.add_root_node(known_hash.clone());
                    Ok(())
                })
                .expect("Failed to change state");
            au.complete().expect("Failed to complete atomic update");
        }
        {
            let state = StorageState::load_state(repository_path).expect("Failed to load state");
            assert_eq!(state.root_snapshots.len(), 1);
            assert_eq!(state.root_snapshots[0], latest_hash);
        }
        {
            let mut state = StorageState::load_state(repository_path).expect("Failed to load state");
            let mut au = AtomicUpdate::new(working_path, repository_path);
            state
                .change_state(&mut au, |state| {
                    state.remove_root_node(&known_hash);
                    Ok(())
                })
                .expect("Failed to change state and remove root snapshot");
            au.complete().expect("Failed to complete atomic update");
            assert_eq!(state.root_snapshots.len(), 0);
        }
        {
            let state = StorageState::load_state(repository_path).expect("Failed to load state");
            assert_eq!(state.root_snapshots.len(), 0);
        }
    }

    #[test]
    fn test_add_recent_hashes() {
        // Generate 15 random hashes
        let mut test_hashes = Vec::new();
        for _ in 0..15 {
            let hash = Hash::generate_random_hash();
            test_hashes.push(hash);
        }
        let ts = TestSpace::new();
        let ts2 = ts.create_child();
        let working_path = ts.get_path();
        let repository_path = ts2.get_path();
        {
            let mut initial =
                StorageState::initialize(repository_path).expect("Failed to initialize state");
            let mut au =
                AtomicUpdate::init(repository_path, working_path).expect("Failed to initialize atomic updater");
            // Add the first 10 hashes to the recent list
            initial.change_state(&mut au, |state| {
                for hash in test_hashes.as_slice()[..10].iter() {
                    state.add_recent_snapshot(hash.clone());
                }
                Ok(())
            }).expect("Failed to change state");
            au.complete().expect("Failed to complete atomic update");
            assert_eq!(initial.recent_snapshots.len(), 10);
        }
        {
            let mut state = StorageState::load_state(repository_path).expect("Failed to load state");
            assert_eq!(state.recent_snapshots.len(), 10);
            // Check for those 10 hashes
            for index in 0..state.recent_snapshots.len() {
                assert_eq!(state.recent_snapshots[index], test_hashes[index]);
            }
            let mut au = AtomicUpdate::new(working_path, repository_path);
            state
                .change_state(&mut au, |state_data| {
                    // Add the final 5 hashes
                    for hash in test_hashes.as_slice()[10..].iter() {
                        state_data.add_recent_snapshot(hash.clone());
                    }
                    Ok(())
                })
                .expect("Failed to change state");
            au.complete().expect("Failed to complete atomic update");
        }
        {
            let state = StorageState::load_state(repository_path).expect("Failed to load state");
            // Check state after removing oldest 5 - so fifth test hash should be first hash in recent
            for index in 5..state.recent_snapshots.len() {
                assert_eq!(state.recent_snapshots[index - 5], test_hashes[index]);
            }
        }
    }

    #[test]
    fn add_remove_end_node_test() {
        let known_hash: [u8; 64] = [
            0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11,
            0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF,
            0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11,
            0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF,
            0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF,
        ];
        let latest_hash = Hash::from(known_hash.as_ref());
        let ts = TestSpace::new();
        let ts2 = ts.create_child();
        let working_path = ts.get_path();
        let repository_path = ts2.get_path();
        let known_hash = latest_hash.clone();
        {
            let mut initial =
                StorageState::initialize(repository_path).expect("Failed to initialize state");
            let mut au =
                AtomicUpdate::init(repository_path, working_path).expect("Failed to initialize atomic updater");
            initial
                .change_state(&mut au, |state| {
                    state.add_end_node(known_hash.clone());
                    Ok(())
                })
                .expect("Failed to change state");
            au.complete().expect("Failed to complete atomic update");
        }
        {
            let state = StorageState::load_state(repository_path).expect("Failed to load state");
            assert_eq!(state.end_snapshots.len(), 1);
            assert_eq!(state.end_snapshots[0], latest_hash);
        }
        {
            let mut state = StorageState::load_state(repository_path).expect("Failed to load state");
            let mut au = AtomicUpdate::new(working_path, repository_path);
            state.change_state(&mut au, |state| {
                state.remove_end_node(&known_hash);
                Ok(())
            }).expect("Failed to change state and remove root snapshot");
            au.complete().expect("Failed to complete atomic update");
            assert_eq!(state.end_snapshots.len(), 0);
        }
        {
            let state = StorageState::load_state(repository_path).expect("Failed to load state");
            assert_eq!(state.end_snapshots.len(), 0);
        }
    }
}
