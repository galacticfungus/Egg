use crate::error::Error;
use crate::storage;
use std::path;
// Equivilant to staging area

type Result<T> = std::result::Result<T, Error>;

/// The storage system is used to access local/ file? storage
pub(crate) struct StorageSystem {
    version: u16,
    storage_file: path::PathBuf,
    // List of stored files
}

impl StorageSystem {
    const VERSION: u16 = 1;

    pub fn initialize_storage() -> Result<StorageSystem> {
        let system = StorageSystem {
            version: StorageSystem::VERSION,
            storage_file: Default::default(),
        };
        Ok(system)
    }
}
