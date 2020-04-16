use crate::hash;
use std::path;
use std::result;
use std::fs;
use std::io::{self};
use byteorder::{self, WriteBytesExt, ReadBytesExt};
use crate::AtomicUpdate;
use crate::snapshots::types::Snapshot;
use crate::error::{Error, UnderlyingError};

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub struct LocalFileStorage {
    current_version: u16, // All files in the repository are stored in this version
    path_to_file_storage: path::PathBuf,
}

// StorageBitField
// TODO: A file doesn't have to be mergable to have delta compression
// 0 - File is mergeable - ie the file will have previous data fragments associated with it
// 1 - File is Packed
// 2 - File is compressed

// Storage - Previous File (Hash) - some forms of delta compression rely on access to the previous version
//           of the file


impl LocalFileStorage {
    const VERSION: u16 = 1;
    // file name of the RepositoryData file
    const FILE_NAME: &'static str = "data_store_state";
    // Path to the directory where files are stored
    pub(crate) const DIRECTORY: &'static str = "STORAGE";

    pub fn initialize(repository_path: &path::Path) -> Result<LocalFileStorage> {
        // TODO: Need additional sensible defaults
        // TODO: This should be using atomic
        let path_to_file = repository_path.join(LocalFileStorage::FILE_NAME);
        let file = match fs::OpenOptions::new().create_new(true).write(true).open(path_to_file.as_path()) {
            Ok(file) => file,
            Err(error) => return Err(Error::file_error(Some(UnderlyingError::from(error)))
                .add_debug_message(format!("Failed to create a data storage state file, path of the file was {}", path_to_file.display()))
                .add_user_message(format!("Failed to initialize part of the repository, the file '{}' could not be created", path_to_file.display()))),
        };
        // Write the version of the local storage
        let mut data_writer = io::BufWriter::new(file);
        if let Err(error) = data_writer.write_u16::<byteorder::LittleEndian>(LocalFileStorage::VERSION) {
            return Err(Error::write_error(Some(UnderlyingError::from(error)))
                .add_debug_message(format!("Failed to write the version of the data storage state file")))
        };
        let path_to_storage = repository_path.join(LocalFileStorage::DIRECTORY);
        if path_to_storage.exists() == false {
            if let Err(error) = fs::create_dir(path_to_storage.as_path()) {
                return Err(Error::file_error(Some(UnderlyingError::from(error)))
                    .add_debug_message(format!("Failed to create the storage directory, path was {}", path_to_storage.display()))
                    .add_user_message(format!("Failed to initialize part of the repository, the directory {} could not be created", path_to_storage.display())));
            }
        }
        Ok(LocalFileStorage {
            current_version: LocalFileStorage::VERSION,
            path_to_file_storage: path_to_storage,
        })
    }

    pub fn load(repository_path: &path::Path) -> Result<LocalFileStorage> {
        // TODO: Correct upgrade path and remove current version from struct
        let storage_config = repository_path.join(LocalFileStorage::FILE_NAME);
        let file = match fs::OpenOptions::new().read(true).open(storage_config.as_path()) {
            Ok(file) => file,
            Err(error) => return Err(Error::file_error(Some(UnderlyingError::from(error)))
                .add_debug_message(format!("Failed to open the local storage state file, path was {}", storage_config.display()))
                .add_user_message(format!("Repository appears to be invalid, a file could not be opened, path was {}", storage_config.display()))),
        };
        let mut storage_reader = io::BufReader::new(file);
        let version = match storage_reader.read_u16::<byteorder::LittleEndian>() {
            Ok(version) => version,
            Err(error) => return Err(Error::parsing_error(Some(UnderlyingError::from(error)))
                .add_debug_message(format!("Failed to read the local storage version"))
                .add_user_message(format!("Failed to read from a file that appears to have become corrupted, path was {}", storage_config.display()))),
        };
        if version == LocalFileStorage::VERSION {
            Ok(LocalFileStorage {
                current_version: 1,
                path_to_file_storage: repository_path.join(LocalFileStorage::DIRECTORY),
            })
        } else {
            // Upgrade path
            unimplemented!("Upgrade path for RepositoryData")
        }
    }
}

impl LocalFileStorage {
    /// Queues all the files associated with a snapshot to be stored
    pub fn store_snapshot(&self, snapshot: &Snapshot, atomic: &mut AtomicUpdate) -> Result<()> {
        for file_to_snapshot in snapshot.get_files() {
            let hash = file_to_snapshot.hash();
            if self.is_file_stored(hash) == false {
                let file_name = String::from(hash);
                let path_to_store = atomic.queue_store(file_name).map_err(|err| err.add_generic_message("During a store snapshot operation"))?;
                if let Err(error) = self.store_file(file_to_snapshot.path(), path_to_store.as_path()) {
                    unimplemented!();
                }
            }
        }
        Ok(())
    }

    /// TODO: Can this be a private method
    pub fn store_file(&self, path_to_file: &path::Path, path_to_store_file: &path::Path) -> Result<()> {
        // Initially just a simple copy
        // TODO: Compression, delta compression etc
        if let Err(error) = fs::copy(path_to_file, path_to_store_file) {
            return Err(Error::parsing_error(Some(UnderlyingError::from(error)))
                .add_debug_message(format!("File copy failed when storing a data file")));
        }
        Ok(())
    }

    pub fn restore_file(&self, file_hash: &hash::Hash, target_path: &path::Path) -> Result<()> {
        // Initially just a simple copy
        // TODO: Hash is used to find the correct file to restore, however I could do this outside the file storage, ie snapshot index or other
        // TODO: the original path will be available to the snapshot system
        // TODO: This is where decompression occurs as well as any needed additional processing to return the version that was placed in this snapshot
        // TODO: Currently restore file assumes that the original is stored in the storage sub directory of the repository.
        let target_file = self.path_to_file_storage.join(String::from(file_hash));
        if let Err(error) = fs::copy(target_file, target_path) {
            return Err(Error::parsing_error(Some(UnderlyingError::from(error)))
                .add_debug_message(format!("File copy failed when restoring a file that had been stored")));
        }
        Ok(())
    }

    pub fn is_file_stored(&self, file_hash: &hash::Hash) -> bool {
        let stored_file_path = self.path_to_file_storage.join(String::from(file_hash));
        stored_file_path.exists()
    }
}

#[cfg(test)]
mod tests {
    use crate::hash::{Hash};
    use testspace;
    use testspace::TestSpace;
    use crate::storage::LocalFileStorage;
    use std::path;

    #[test]
    fn test_store_file() {
        let ts = TestSpace::new();
        let mut tsf = ts.create_tsf();
        tsf.write_random_bytes(2048);
        let base_path = ts.get_path();
        let rep_data = LocalFileStorage::initialize(base_path).expect("Failed to initialize RepositoryData");
        let path_to_file = tsf.get_path();
        let hash = Hash::hash_file(path_to_file).expect("Failed to hash file");
        let file_name = String::from(&hash);
        let target_path = base_path.join(LocalFileStorage::DIRECTORY).join(file_name);
        rep_data.store_file(path_to_file, target_path.as_path()).expect("Failed to store file");
        let result = rep_data.is_file_stored(&hash);
        assert!(result);
        let stored_file_name = path::Path::new(LocalFileStorage::DIRECTORY).join(String::from(hash));
        let expected_path = base_path.join(stored_file_name);
        assert!(expected_path.exists());
    }

    #[test]
    fn test_retrieve_file() {
        let ts= TestSpace::new();
        let mut tsf = ts.create_tsf();
        tsf.write_random_bytes(1024);
        let repository_path = ts.get_path();
        let file_to_store = tsf.get_path();
        let data = LocalFileStorage::initialize(repository_path).expect("Failed to initialize repository data");
        let hash = Hash::hash_file(file_to_store).expect("Failed to hash file");
        let file_name = String::from(&hash);
        let path_of_file = repository_path.join(LocalFileStorage::DIRECTORY).join(file_name);
        data.store_file(file_to_store, path_of_file.as_path()).expect("Failed to store file");
        let restore_path = repository_path.join("target_file");
        data.restore_file(&hash, restore_path.as_path()).expect("Failed to restore file");
        assert!(restore_path.exists());
        let restored_hash = Hash::hash_file(file_to_store).expect("Failed to hash restored file");
        assert_eq!(hash, restored_hash);
    }
}
