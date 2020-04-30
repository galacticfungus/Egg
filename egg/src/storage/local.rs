use std::{path, result, fs, io};

use byteorder::{self, WriteBytesExt, ReadBytesExt};

use super::LocalStorage;
use crate::hash;
use crate::AtomicUpdate;
use crate::snapshots::Snapshot;
use crate::error::{Error, UnderlyingError};

// StorageBitField
// 0 - File is merged - ie the file is compressed (delta) based on previous versions of this file
// 1 - File is Packed
// 2 - File is compressed


impl LocalStorage {
    const VERSION: u16 = 1;
    // file name of the RepositoryData file
    const FILE_NAME: &'static str = "state";
    // Path to the directory where files are stored
    pub(crate) const DIRECTORY: &'static str = "storage";

    pub fn initialize(repository_path: &path::Path) -> Result<LocalStorage, Error> {
        // TODO: Need additional sensible defaults
        let path_to_storage = repository_path.join(LocalStorage::DIRECTORY);
        if path_to_storage.exists() == false {
            if let Err(error) = fs::create_dir(path_to_storage.as_path()) {
                return Err(Error::file_error(Some(UnderlyingError::from(error)))
                    .add_debug_message(format!("Failed to create the storage directory, path was {}", path_to_storage.display()))
                    .add_user_message(format!("Failed to initialize part of the repository, the directory {} could not be created", path_to_storage.display())));
            }
        }
        let path_to_file = path_to_storage.join(Self::FILE_NAME);
        let file = match fs::OpenOptions::new().create_new(true).write(true).open(path_to_file.as_path()) {
            Ok(file) => file,
            Err(error) => return Err(Error::file_error(Some(UnderlyingError::from(error)))
                .add_debug_message(format!("Failed to create a data storage state file, path of the file was {}", path_to_file.display()))
                .add_user_message(format!("Failed to initialize part of the repository, the file '{}' could not be created", path_to_file.display()))),
        };
        // Write the version of the local storage
        let mut data_writer = io::BufWriter::new(file);
        if let Err(error) = data_writer.write_u16::<byteorder::LittleEndian>(LocalStorage::VERSION) {
            return Err(Error::write_error(Some(UnderlyingError::from(error)))
                .add_debug_message(format!("Failed to write the version of the data storage state file")))
        };
        Ok(LocalStorage {
            path_to_file_storage: path_to_storage,
        })
    }

    pub fn load(repository_path: &path::Path) -> Result<LocalStorage, Error> {
        // TODO: Correct upgrade path and remove current version from struct
        let storage_config = repository_path.join(LocalStorage::DIRECTORY).join(LocalStorage::FILE_NAME);
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
        if version == LocalStorage::VERSION {
            Ok(LocalStorage {
                path_to_file_storage: repository_path.join(LocalStorage::DIRECTORY),
            })
        } else {
            // Upgrade path
            unimplemented!("Upgrade path for LocalStorage")
        }
    }
}

impl LocalStorage {
    // TODO: Large files greater than 1MB to 10MB should be split into multiple files?
    // NOTE: Files need some self imposed hiearchy however that hiearchy cannot be centralized and must grow organically
    /// Queues all the files associated with a snapshot to be stored
    pub fn store_snapshot(&self, snapshot: &Snapshot, atomic: &mut AtomicUpdate) -> Result<(), Error> {
        // TODO: Filter out files that didn't change
        // TODO: Delta compression
        // TODO: Classify each file ie new, changed, renamed, removed
        // TODO: Include the parent snapshot
        // TODO: Track hiearchy at the file level
        for file_to_snapshot in snapshot.get_files() {
            let hash = file_to_snapshot.hash();
            // FIXME: There are certain problems with this approach, for instance if a file is modified serveral times of several snapshots and then reverts back to the old version then the system has no way of knowing that it was a reversion unless we track this at the snapshot level
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

    fn store_file(&self, path_to_file: &path::Path, path_to_store_file: &path::Path) -> Result<(), Error> {
        // Initially just a simple copy
        // TODO: Compression, delta compression etc
        if let Err(error) = fs::copy(path_to_file, path_to_store_file) {
            return Err(Error::parsing_error(Some(UnderlyingError::from(error)))
                .add_debug_message(format!("File copy failed when storing a file")));
        }
        Ok(())
    }

    pub fn restore_file(&self, file_hash: &hash::Hash, target_path: &path::Path) -> Result<(), Error> {
        // Initially just a simple copy
        // TODO: Hash is used to find the correct file to restore, however I could do this outside the file storage, ie snapshot index or other
        // TODO: the original path will be available to the snapshot system
        // TODO: This is where decompression occurs as well as any needed additional processing to return the version that was placed in this snapshot
        // TODO: Currently restore file assumes that the original is stored in the storage sub directory of the repository.
        let target_file = self.path_to_file_storage.join(String::from(file_hash));
        if let Err(error) = fs::copy(target_file, target_path) {
            return Err(Error::parsing_error(Some(UnderlyingError::from(error)))
                .add_debug_message(format!("File copy failed when restoring a file that had been stored in local storage")));
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
    use crate::storage::LocalStorage;
    use std::path;

    #[test]
    fn test_store_file() {
        let ts = TestSpace::new();
        let mut tsf = ts.create_tsf();
        tsf.write_random_bytes(2048);
        let base_path = ts.get_path();
        let rep_data = LocalStorage::initialize(base_path).expect("Failed to initialize RepositoryData");
        let path_to_file = tsf.get_path();
        let hash = Hash::hash_file(path_to_file).expect("Failed to hash file");
        let file_name = String::from(&hash);
        let target_path = base_path.join(LocalStorage::DIRECTORY).join(file_name);
        rep_data.store_file(path_to_file, target_path.as_path()).expect("Failed to store file");
        let result = rep_data.is_file_stored(&hash);
        assert!(result);
        let stored_file_name = path::Path::new(LocalStorage::DIRECTORY).join(String::from(hash));
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
        let data = LocalStorage::initialize(repository_path).expect("Failed to initialize repository data");
        let hash = Hash::hash_file(file_to_store).expect("Failed to hash file");
        let file_name = String::from(&hash);
        let path_of_file = repository_path.join(LocalStorage::DIRECTORY).join(file_name);
        data.store_file(file_to_store, path_of_file.as_path()).expect("Failed to store file");
        let restore_path = repository_path.join("target_file");
        data.restore_file(&hash, restore_path.as_path()).expect("Failed to restore file");
        assert!(restore_path.exists());
        let restored_hash = Hash::hash_file(file_to_store).expect("Failed to hash restored file");
        assert_eq!(hash, restored_hash);
    }
}
