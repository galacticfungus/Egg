use crate::snapshots;
use byteorder::{self, LittleEndian, ReadBytesExt, WriteBytesExt};
use std::fs;
use std::io::{BufReader, BufWriter};
use std::path;
use crate::storage::LocalFileStorage;
use crate::snapshots::RepositorySnapshots;
use crate::snapshots::types::{SnapshotId, Snapshot};
use crate::error::{ Error, UnderlyingError };


#[derive(Debug)]
pub struct Repository {
    version: u16,
    /// Working Path is the path to the directory that contains the repository
    working_path: path::PathBuf,
    egg_path: path::PathBuf,
    version_file: path::PathBuf,
    snapshot_storage: RepositorySnapshots,
    data: LocalFileStorage,
}

// 
type Result<T> = std::result::Result<T, Error>;

// Public interfaces
impl Repository {
    const EGG_VERSION: u16 = 1;
    /// Returns an egg struct representing the repository that contains the given path, returns an
    /// error (EggKind::RepositoryNotFound) if one could not be found
    pub fn find_egg(path: &path::Path) -> Result<Repository> {
        // this only needs to search the given path and up the directory tree since we are only interested in files and
        // directories that could be contained within the repository
        // TODO: Testing
        // TODO: Replace these literals with constants defined in Egg
        if let Some(working_path) = Repository::search_up_tree(path)? {
            println!("Path returned from search up tree: {}", working_path.display());
            let repository_path = working_path.join(".egg");
            let version_file = repository_path.join("version");
            let version = match Repository::read_version_file(version_file.as_path()) {
                Ok(version) => version,
                Err(error) => return Err(error.add_debug_message(format!("Could not read the repositories version file, path to the file was {}", version_file.display()))
                                              .add_user_message(format!("Failed to verify the version of the repository at {}, this is most likely the result of a bug or third part modification", repository_path.display()))),
            };
            if version != Repository::EGG_VERSION {
                // Upgrade path
                unimplemented!("Hit repository upgrade path");
            }
            // TODO: Load snapshots
            let snapshot_storage = match snapshots::RepositorySnapshots::restore(working_path.as_path(), repository_path.as_path()) {
                Ok(snapshot_storage) => snapshot_storage,
                Err(error) => return Err(error.add_debug_message(format!("Failed to restore the current state of the snapshot system"))
                                              .add_user_message(format!("The repository appears to be corrupt, this is due either to a bug or third party modification"))),
            };
            // TODO: Load RepositoryData
            let file_data = LocalFileStorage::load(repository_path.as_path())?;
            let egg = Repository {
                version: Repository::EGG_VERSION,
                egg_path: repository_path,
                version_file,
                snapshot_storage,
                working_path,
                data: file_data
            };
            return Ok(egg);
        }
        // No egg found so return error
        Err(Error::repository_not_found()
            .add_debug_message(format!("find_egg failed to find a repository that contained the path {}", path.display()))
            .add_user_message(format!("The path {} is not located within a repository", path.display())))
    }
    /// Create a repository at the given path, the path must already exist and must be a directory,
    /// the version of the repository created is always the latest
    pub fn create_repository(path: &path::Path) -> Result<Repository> {
        // TODO: Allow for a path that doesn't exist
        // Does the path exist
        // TODO: This probably needs to be a metadata as errors are just coerced to false
        if path.exists() == false {
            if let Err(error) = fs::create_dir_all(path) {
                return Err(Error::file_error(Some(UnderlyingError::from(error)))
                    .add_debug_message(format!("Failed to create the path {} when trying to create a new repository", path.display()))
                    .add_user_message(format!("Failed to create the new repository, the path {} could not be created", path.display())));
            }
        } else if path.is_dir() == false {
            return Err(Error::file_error(None)
                    .add_debug_message(format!("The path {} given to the create_repository function existed and was not a directory", path.display()))
                    .add_user_message(format!("The path {} does not point to a directory, could not create a repository there", path.display())));
        }
        // TODO: Ensure that the path is converted to an absolute path here, to work around path limitation on windows
        // TODO: Seperate module for path that on windows ensures that they are UNC paths and relative on linux, viable?
        let absolute_path = match path.canonicalize() {
            Ok(path) => path,
            Err(error) => return Err(Error::file_error(Some(UnderlyingError::from(error)))
                .add_generic_message(format!("Failed to convert the path {} to an absolute path", path.display()))),
        };
        // It is an error to find a repository that is either a sub directory of this directory or
        // a parent of this directory
        // Check if a repository is in a sub directory of this path
        if let Some(repository_path) = Repository::search_down_tree(absolute_path.as_path())? {
            // return build_error!(ErrorKind::RepositoryAlreadyExists,
            //     "At least one repository was found in a subdirectory at {}, which is a subdirectory of {}, egg repositories can't contain another egg repository",
            //     repository_path.display(),
            //     path.display());
            return Err(Error::file_error(None)
                .add_generic_message(
                    format!("Error when creating repository at {0}, at least one repository was found in a subdirectory of {0} at {1}, egg repositories can't contain another egg repository",
                        path.display(),
                        repository_path.display(),
                    )));
        }
        // Check if there is a repository containing this path
        if let Some(repository_path) = Repository::search_up_tree(absolute_path.as_path())? {
            return Err(Error::file_error(None)
                .add_generic_message(format!("Error when creating repository at {}, a repository was found in a parent directory at {}, a repository cannot contain another repository",
                    path.display(),
                    repository_path.display(),
                )));
        }
        // Path must be valid at this point
        // Create basic folder structure and version information for this repository
        let egg = match Repository::initialize_repository(absolute_path) {
            Ok(egg) => egg,
            Err(error) => return Err(error.add_debug_message(format!("Error occured when initializing a repository at {}, the initialize_repository function returned an error", path.display()))
                                          .add_user_message(format!("While the path {} appeared to be valid, a system error occurred while trying to create the repository", path.display()))),
        };
        Ok(egg)
    }
}

impl Repository {
    /// Takes a snapshot
    pub fn take_snapshot<S: Into<String>>(&mut self, parent: Option<SnapshotId>, snapshot_message: S, files_to_snapshot: Vec<path::PathBuf>) -> Result<SnapshotId> {
        // let snapshot = self.snapshot_state.take_snapshot(parent, snapshot_message, files_to_snapshot, self.egg_path.as_path(), &self.data);
        // Paths to the files to be snapshotted must be canonicalized
        let mut validated_paths = Vec::new();
        for file_to_snapshot in files_to_snapshot {
            let validated_path = match file_to_snapshot.canonicalize() {
                Ok(validated_path) => validated_path,
                Err(error) => return Err(Error::file_error(Some(UnderlyingError::Io(error)))
                    .add_generic_message(format!("Failed to convert the path {} to an absolute path", file_to_snapshot.display()))),
            };
            // return Err(Error::file_error(Some(UnderlyingError::from(error)))
            // .add_generic_message())

            validated_paths.push(validated_path);
        }
    
        let fs = match LocalFileStorage::load(self.egg_path.as_path()) {
            Ok(fs) => fs,
            Err(error) => return Err(error.add_debug_message(format!("Failed to load the state of the files being stored in the repository at {}", self.egg_path.display()))
                                          .add_user_message("Taking a snapshot failed, the repository appears to be corrupted from either a bug or due to third party modification")),
        };
        // let hashed_files = crate::hash::Hash::hash_file_list(files_to_snapshot);
        let created_id = match self.snapshot_storage.take_snapshot(parent, snapshot_message.into(), validated_paths, self.egg_path.as_path(), self.working_path.as_path(), &fs) {
            Ok(id) => id,
            Err(error) => return Err(error.add_debug_message(format!("take_snapshot in the snapshot module returned an error"))
                                          .add_user_message(format!("There was a problem when trying to take a snapshot"))),
        };
        Ok(created_id)
    }
    // Take a snapshot based on the active snapshot - creates a snapshot with the active snapshot as a parent
  pub fn take_incremental_snapshot<S: Into<String>>(&mut self, snapshot_message: S, files_to_snapshot: Vec<path::PathBuf>) -> Result<()> {
    // TODO: Get current snapshot

    Ok(())
  }
  /// Create a snapshot of all the files that have changed
  pub fn take_snapshot_of_changes<S: Into<String>>(&mut self, snapshot_message: S, files_to_snapshot: Vec<path::PathBuf>) -> Result<()> {
    // TODO: Get current tracked files
    // self.snapshot_state.take_snapshot(None, snapshot_message, files_to_snapshot, self.egg_path.as_path(), &self.data);
    Ok(())
  }
  /// Create a snapshot based on the active snapshot of all the files that have changed
  pub fn take_incremental_snapshot_of_changes<S: Into<String>>(&mut self, snapshot_message: S, files_to_snapshot: Vec<path::PathBuf>) -> Result<()> {
    // TODO: Get current snapshot
    // TODO: Get current files
    // self.snapshot_state.take_snapshot(None, snapshot_message, files_to_snapshot, self.egg_path.as_path(), &self.data);
    Ok(())
  }
}

impl Repository {
    pub fn get_latest_snapshot(&self) -> Option<SnapshotId> {
        self.snapshot_storage.get_latest_snapshot()
    }
}

impl Repository {
    pub fn get_snapshot(&self, snapshot_id: SnapshotId) -> Option<&Snapshot> {
        self.snapshot_storage.get_snapshot_by_id(snapshot_id, self.egg_path.as_path(), self.working_path.as_path())
    }


}

impl Repository {
    /// Searches the path and all parent directories for a repository, returns a path if
    /// a repository was found or None if one wasn't found. The path returned is the working directory not the path to the
    /// repository itself.
    fn search_up_tree(dir_to_search: &path::Path) -> Result<Option<path::PathBuf>> {
        // It's not necessary for the path to point to a directory, however, it is necessary to create an absolute path
        let mut absolute_path = match dir_to_search.canonicalize() {
            Ok(absolute_path) => absolute_path,
            Err(error) => return Err(Error::file_error(Some(UnderlyingError::from(error)))
                .add_debug_message(format!("Failed to convert a relative path to a canonicalized path, the path that could not be converted was {}", dir_to_search.display()))
                .add_user_message(format!("An invalid path (it had invalid characters) was found when scanning the directory structure, path was {}", dir_to_search.display()))),
        };
        absolute_path.push(".egg");
        if absolute_path.exists() {
            //TODO: Validate that it is actually an egg and a directory
            absolute_path.pop();
            println!("Repository found at {}", absolute_path.display());
            return Ok(Some(absolute_path));
        }
        while absolute_path.pop() {
            // Check for a .egg directory in this directory
            absolute_path.push(".egg");
            if absolute_path.exists() {
                // TODO: Validate that it is actually an egg and a directory
                absolute_path.pop();
                println!("Repository found at {}", absolute_path.display());
                return Ok(Some(absolute_path));
            }
            // Remove .egg
            absolute_path.pop();
        }
        Ok(None)
    }

    /// Searches all sub directories of the specified directory looking for a egg repository, returns
    /// the repositories path.
    fn search_down_tree(dir_to_search: &path::Path) -> Result<Option<path::PathBuf>> {
        // Create a list of directories to search
        let mut directories_to_search = Vec::new();
        // Start with the current one
        // TODO: Don't need to search the current directory as search_up_for_repository checks that
        directories_to_search.push(dir_to_search.to_path_buf());
        // Loop through each directory we find and see if it contains a repository
        while let Some(directory) = directories_to_search.pop() {
            // Check if this path contains a repository
            let repository_path = directory.join(".egg");
            if repository_path.exists() {
                //TODO: Verify it's a valid repository
                return Ok(Some(repository_path));
            }
            // Find all the subdirectories of the current directory
            let read_result = match fs::read_dir(directory.as_path()) {
                Ok(read_result) => read_result,
                Err(error) => return Err(Error::file_error(Some(UnderlyingError::from(error)))
                    .add_debug_message(format!("Failed searching directory tree, could not read the contents of the directory {}", directory.display()))
                    .add_user_message(format!("Could not read the contents of the directory {}", directory.display()))),
            };
            // Loop through them adding them to the list of directories to search
            for dir_entry in read_result {
                // Is the directory entry valid
                let valid_dir_entry = match dir_entry {
                    Ok(dir_entry) => dir_entry,
                    Err(error) => return Err(Error::file_error(Some(UnderlyingError::from(error)))
                        .add_debug_message(format!("Failed searching directory tree, one of the directory entries for the path {} could not be read", directory.display()))
                        .add_user_message(format!("Could not read the contents of the directory {}", directory.display()))),
                };
                let file_type = match valid_dir_entry.file_type() {
                    Ok(file_type) => file_type,
                    Err(error) => return Err(Error::file_error(Some(UnderlyingError::from(error)))
                        .add_generic_message(format!("Failed searching directory tree, could not determine if this path was a directory {}", valid_dir_entry.path().display()))),
                };
                // If the entry is a directory, add it to the list of items to search
                if file_type.is_dir() {
                    directories_to_search.push(valid_dir_entry.path());
                }
            }
        }
        // No Repository found
        Ok(None)
    }

    /// Verifies that the directory contains a MAYBE valid egg repository, validation only goes so far
    /// as to verify the version of the repository and a valid pointer to a core file.
    fn verify_egg(path_to_egg: &path::Path) -> Result<bool> {
        unimplemented!();
        // TODO: Check for a egg file and the existence of a version file, the actual version is checked when the repository is loaded
    }

    // Creates the infrastructure needed to identify the current version of a repository, not the data
    // structures that version may rely on, it also creates the bare minimum directory structure for
    // a repository, the path passed to this function should be the proposed working directory of
    // the repository and not the egg directory itself
    fn initialize_repository<P: Into<path::PathBuf>>(path: P) -> Result<Repository> {
        let path = path.into();
        let repository_path = path.join(".egg");
        if let Err(error) = fs::create_dir(repository_path.as_path()) {
            return Err(Error::file_error(Some(UnderlyingError::from(error)))
                .add_generic_message(format!("Failed to create a directory when initializing the repository, path was {}", repository_path.display())));
        }
        // Path to version file
        let version_path = repository_path.join("version");
        // Write a current version file
        Repository::write_version_file(version_path.as_path())?;
        //TODO: Move these initialization calls to the create_repository function
        // Initialize a new snapshot file
        let snapshot_storage = RepositorySnapshots::initialize(repository_path.as_path(), path.as_path())?;

        let data = LocalFileStorage::initialize(repository_path.as_path())?;
        Ok(Repository {
            version: Repository::EGG_VERSION,
            working_path: path,
            egg_path: repository_path,
            version_file: version_path,
            snapshot_storage,
            data
        })
    }

    /// Writes a current version file, a version file enables changing the underlying structure of a
    /// egg repository completely while remaining backwards compatible, it contains nothing but a version number
    fn write_version_file(path_to_file: &path::Path) -> Result<()> {
        let version_file = match fs::OpenOptions::new().write(true).create_new(true).open(path_to_file) {
            Ok(file) => file,
            Err(error) => return Err(Error::file_error(Some(UnderlyingError::from(error)))
                            .add_user_message(format!("Failed to open a file, path was {}", path_to_file.display()))
                            .add_debug_message(format!("Failed to open a file when trying to write a new version file, path was {}", 
                                path_to_file.display()))),
            
        };
        // TODO: Create a byte signature to help verify a repository quickly
        let mut writer = BufWriter::new(version_file);
        if let Err(error) = writer.write_u16::<byteorder::LittleEndian>(Repository::EGG_VERSION) {
            return Err(Error::write_error(Some(UnderlyingError::from(error)))
                .add_user_message(format!("Failed to write data to a new file, path was {}", path_to_file.display()))
                .add_debug_message(format!("Failed to write the version of the repository to a new version file, path was {}", 
                    path_to_file.display()))
            );
        }
        Ok(())
    }

    /// Reads a version file and returns the version number
    fn read_version_file(path_to_file: &path::Path) -> Result<u16> {
        let file = match fs::OpenOptions::new().read(true).open(path_to_file) {
            Ok(file) => file,
            Err(error) => {
                return Err(Error::file_error(Some(UnderlyingError::from(error)))
                    .add_user_message(format!("Failed to open a file, path was {}", path_to_file.display()))
                    .add_debug_message(format!("Failed to open a file when trying to write a new version file, path was {}", 
                        path_to_file.display()))
                );
            }
        };
        let mut reader = BufReader::new(file);
        // TODO: Read the byte signature
        let version = match reader.read_u16::<LittleEndian>() {
            Ok(version) => version,
            Err(error) => {
                return Err(Error::file_error(Some(UnderlyingError::from(error)))
                    .add_user_message(format!("Failed to read data from a file, path was {}", path_to_file.display()))
                    .add_debug_message(format!("Failed to read the version number from the version file, path was {}", 
                        path_to_file.display()))
                );
            }
        };
        Ok(version)
    }

    pub fn get_working_path(&self) -> &path::Path {
        self.working_path.as_path()
    }
}

#[cfg(test)]
#[macro_use]
mod tests {
    use super::Repository;
    use testspace::{TestSpace};

    // Only need this for testing
    impl PartialEq for Repository {
        fn eq(&self, other: &Repository) -> bool {
            if self.version != other.version {
                return false;
            }
            if self.working_path != other.working_path {
                return false;
            }
            return true;
        }
    }

    // Previous versions of repository functions
    impl Repository {
        //TODO: Versioned functions for initializing a repository for testing reading older versions of the repository
        
    }

    #[test]
    fn init_repository_test() {
        let ts = TestSpace::new();
        let temp_path = ts.get_path();
        let egg = Repository::initialize_repository(temp_path).unwrap();
        assert_eq!(egg.version, Repository::EGG_VERSION);
        assert!(egg.egg_path.exists());
        assert!(egg.version_file.exists());
    }

    #[test]
    fn fail_search_down_tree_test() {
        let ts = TestSpace::new();
        let ts2 = ts.create_child();
        let ts3 = ts.create_child();
        let ts4 = ts2.create_child();
        let ts5 = ts3.create_child();
        let ts6 = ts3.create_child();
        let _ts7 = ts5.create_child();
        let _ts8 = ts5.create_child();
        let _ts9 = ts4.create_child();
        let _ts10 = ts4.create_child();
        let _ts11 = ts6.create_child();
        let _ts12 = ts6.create_child();
        if let Some(path) = Repository::search_down_tree(ts.get_path()).unwrap() {
            panic!("Test Failed: Found an invalid egg at {}", path.display());
        }
    }

    #[test]
    fn search_down_tree_test() {
        let ts = TestSpace::new();
        let ts2 = ts.create_child();
        let ts3 = ts.create_child();
        let ts4 = ts2.create_child();
        let ts5 = ts3.create_child();
        let ts6 = ts3.create_child();
        let _ts7 = ts5.create_child();
        let _ts8 = ts5.create_child();
        let _ts9 = ts4.create_child();
        let _ts10 = ts4.create_child();
        let mut ts11 = ts6.create_child();
        ts11.create_dir(".egg");
        let _ts12 = ts6.create_child();
        if let None = Repository::search_down_tree(ts.get_path()).unwrap() {
            panic!("Test Failed: Didn't find a valid egg");
        }
    }

    // The test runs on a simple absolute path
    #[test]
    fn find_egg_simple_test() {
        let ts = TestSpace::new();
        let ts2 = ts.create_child();
        // TODO: Need to create a repository to do this test now since an egg can't be created without validation as well as a snapshot system and repositorydata
        Repository::create_repository(ts.get_path()).expect("Failed to create repository");
        let result = match Repository::find_egg(ts2.get_path()) {
            Ok(egg) => egg,
            Err(error) => panic!("Test test_find_egg Failed, error was {}", error),
        };
        let expected = ts.get_path().canonicalize().unwrap();
        assert_eq!(result.working_path.as_path(), expected.as_path());
        println!("Final Working Directory: {}", result.working_path.display());
    }

    #[test]
    fn find_no_egg_test() {
        let ts = TestSpace::new();
        let ts2 = ts.create_child();
        let error = Repository::find_egg(ts2.get_path()).unwrap_err();
        // error.
    }

    #[test]
    fn read_write_version_file_test() {
        let ts = TestSpace::new();
        let temp_path = ts.get_path().join("version");
        if let Err(error) = Repository::write_version_file(temp_path.as_path()) {
            panic!(
                "Error occurred while writing the version file, error was {}",
                error
            );
        }
        let version = Repository::read_version_file(temp_path.as_path()).unwrap_or_else(|err| {
            panic!("Failed to read version from version file, error was {}", err);
        });
        assert_eq!(version, Repository::EGG_VERSION);
    }

    

    #[test]
    fn initialized_version_file_test() {
        let ts = TestSpace::new();
        let working_path = ts.get_path();
        let egg = Repository::initialize_repository(working_path).unwrap_or_else( |error| {
            panic!("Failed to initialize an egg repository, error was {}", error);
        });
        // Basic egg repository should have been created
        let egg_path = working_path.join(".egg");
        let version_path = egg_path.join("version");
        assert!(egg_path.is_dir());
        let version = Repository::read_version_file(version_path.as_path()).unwrap_or_else( |error| {
            panic!("Failed to read egg repository version, error was {}", error);
        });
        assert_eq!(version, Repository::EGG_VERSION);
        assert_eq!(egg.version, Repository::EGG_VERSION);
        assert_eq!(egg.working_path, working_path);
    }

    #[test]
    fn egg_take_snapshot_test() {
        unimplemented!();
    }
    #[test]
    fn egg_incremental_snapshot_test() {
        unimplemented!();
    }

    #[test]
    fn egg_take_snapshot_of_changes_test() {
        unimplemented!();
    }

    #[test]
    fn egg_take_incremental_snapshot_of_changes_test() {
        unimplemented!();
    }
}
