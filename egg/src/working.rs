use std::path;
use std::fs;
use std::time::{SystemTime, Duration};
use std::io::{BufWriter, BufReader, Seek, SeekFrom};
use byteorder::{WriteBytesExt, ReadBytesExt};
use byteorder::LittleEndian;
use crate::storage::stream::{ReadEggExt, WriteEggExt};
use std::path::PathBuf;
use crate::error::{Error,UnderlyingError};
use crate::hash::Hash;
use std::collections::HashMap;
use crate::snapshots::types::{Snapshot, FileMetadata};

type Result<T> = std::result::Result<T, Error>;

// TODO: Redo this as a Vec and HashSet


/// Represents a file in the working directory that we may wish to snapshot or otherwise investigate,
/// This structure does not contain the path of the file since the path is used as a key inside a map of WorkingFiles
struct WorkingFile {
    hash: Option<Hash>,
    file_size: u64,
    modified_time: u128,
}

impl WorkingFile {
    pub fn is_hashed(&self) -> bool {
        self.hash.is_some()
    }
    pub fn hash(&self) -> Option<&Hash> {
        self.hash.as_ref()
    }

    pub fn filesize(&self) -> u64 {
        self.file_size
    }

    pub fn modified_time(&self) -> u128 {
        self.modified_time
    }
}

/// Contains a number of helpful functions for dealing with the Repositories working directory
/// Primarily it provides an interface to check if a file(s) have changed since a snapshot was taken
pub struct WorkingDirectory<'a> {
    working_files: HashMap<path::PathBuf, WorkingFile>,
    path_to_working: &'a path::Path,
}

impl<'a> WorkingDirectory<'a> {
    pub fn new(path_to_working: &'a path::Path) -> WorkingDirectory {
        WorkingDirectory {
            working_files: HashMap::new(),
            path_to_working,
        }
    }

    /// Looks at all the files in a directory and stores file size, file name and last modified file time, 
    fn index_directory(path_to_search: &path::Path, files_found: &mut HashMap<path::PathBuf, WorkingFile>, directories_found: &mut Vec<PathBuf>) -> Result<()> {
        let items_found = match fs::read_dir(path_to_search) {
            Ok(result)     => result,
            Err(error) => unimplemented!(),
        };
        for item in items_found {
            let valid_item = match item {
                Ok(valid_item) => valid_item,
                Err(error) => unimplemented!(),
            };
            let path = valid_item.path();
            
            let file_type = match valid_item.file_type() {
                Ok(item_type) => item_type,
                Err(error) => unimplemented!(),
            };
            if file_type.is_dir() {
                // TODO: check if the path is .egg as we need to ignore this
                // TODO: Technically we need to filter out the repository directory, so the .egg folder inside the working directory
                // self.path_to_working.join(".egg");
                directories_found.push(path);
            } else {
                // Add the file to the list
                let metadata = match valid_item.metadata() {
                    Ok(metadata) => metadata,
                    Err(error) => unimplemented!(),
                };
                let modified_time = WorkingDirectory::get_modified_time(&metadata);
                // let time_modified = date.elapsed();
                let data = WorkingFile {
                    hash: None, // We only need to provide a hash if the file system can't provide a length or modification time for the given file
                    file_size: metadata.len(),
                    modified_time,
                };
                files_found.insert(path, data);
            }
        }
        Ok(())
    }

    // Retrieves the last modified time of a file with microsecond resolution
    fn get_modified_time(file_metadata: &fs::Metadata) -> u128 {
        match file_metadata.modified() {
            Ok(valid_date) => match valid_date.duration_since(SystemTime::UNIX_EPOCH) {
                Ok(time_modified) => time_modified.as_micros(),
                Err(error) => unimplemented!(), // This means that the file was modified before the UNIX EPOCH
            },
            Err(error) => unimplemented!(), // File system does not support obtaining the last modified file time - TODO: Fallback to using a hash and display a warning
        }
    }

    fn index_repository(&self) -> Result<HashMap<path::PathBuf, WorkingFile>> {
        // All files must be relative to the working directory as that is how snapshot paths are stored
        let mut directories_to_search = Vec::new();
        let mut files_found = HashMap::new();
        WorkingDirectory::index_directory(self.path_to_working, &mut files_found, &mut directories_to_search)?;
        while directories_to_search.is_empty() == false {
            let path_to_search = directories_to_search.pop().unwrap();
            WorkingDirectory::index_directory(path_to_search.as_path(), &mut files_found, &mut directories_to_search)?;
        }
        Ok(files_found)
    }

    /// Compares the list of hashed paths with the working directory
    pub fn get_changed_files<'b>(&self, stored_files: &'b [FileMetadata]) -> Vec<&'b path::Path> {
        // Get snapshot paths and their hashes
        // Lookup path in changed files and compare hashes
        // Get an update to date list of all files in the repositories working directory
        // let root_dir = ;
        let mut changed_files = Vec::new();
        let files_found = match self.index_repository() {
            Ok(files_found) => files_found,
            Err(error) => unimplemented!(),
        };
        for stored_file in stored_files {
            let working_file = match files_found.get(stored_file.path()) {
                Some(working_file) => working_file,
                None => {
                    // The working directory has no file with that path
                    // TODO: Every changed file needs a status associated with it - ie sometimes a file is deleted renamed or created, as opposed to just edited
                    changed_files.push(stored_file.path());// File exists in list but not in repository
                    break;
                }, 
            };
            if working_file.file_size != stored_file.filesize() {
                // File sizes do not match
                changed_files.push(stored_file.path());
            }
            if working_file.modified_time != stored_file.modified_time() {
                // Time of modification does not match
                changed_files.push(stored_file.path());
            }
        }
        changed_files
    }

    /// Compares the working directory with the hashes stored in a snapshot
    pub fn get_files_changed_since_snapshot<'b>(&self, snapshot: &'b Snapshot, hashed_files: &'b [(path::PathBuf, Hash)]) -> Vec<&'b path::Path> {
        let hashed_paths = snapshot.get_files();
        self.get_changed_files(hashed_paths)
    }

    // // TODO: Move this function into the working module
    // // Takes a vector of paths or pathbufs and returns a vector of tuples containing the path and the hash
    // fn hash_file_list<P: Into<path::PathBuf>>(file_list: Vec<P>) -> Result<Vec<(path::PathBuf, Hash)>> {
    //     // TODO: This should be moved to Hash
    //     let mut path_with_hash = Vec::with_capacity(file_list.len());
    //     // Hash all the files being snapshot and store it along with the path in the snapshot structure
    //     for path_to_file in file_list {
    //         let path_to_file = path_to_file.into();
    //         let hash_string = match Hash::hash_file(path_to_file.as_path()) {
    //             Ok(hash_string) => hash_string,
    //             Err(error) => return Err(error.add_debug_message(format!("Failed to process the list of files to hash, the problem file was {}", path_to_file.display()))),
    //         };
    //         path_with_hash.push((path_to_file, hash_string));
    //     }
    //     Ok(path_with_hash)
    // }

}

impl<'a> WorkingDirectory<'a> {

    /// Given a list of paths, this function returns a list of FileMetadata
    pub(crate) fn create_metadata_list(files_to_store: Vec<path::PathBuf>) -> Result<Vec<FileMetadata>> {
        let mut storage_list = Vec::new();
        for file_to_store in files_to_store {
            let metadata = match WorkingDirectory::get_file_metadata(file_to_store) {
                Ok(metadata) => metadata,
                Err(error) => unimplemented!(),
            };
            storage_list.push(metadata);
        }
        Ok(storage_list)
    }

    /// Gets the file size and time modified of the given path as well as its hash
    /// This is used to collect information about a file that is part of a snapshot
    fn get_file_metadata(path_to_file: path::PathBuf) -> Result<FileMetadata> {
        let hash_of_file = match Hash::hash_file(path_to_file.as_path()) {
            Ok(hash_of_file) => hash_of_file,
            Err(error) => unimplemented!(),
        };
        let file_data = match path_to_file.metadata() {
            Ok(file_data) => file_data,
            Err(error) => unimplemented!(),
        };
        let file_size = file_data.len();
        let metadata = FileMetadata::new(hash_of_file, file_size, path_to_file, WorkingDirectory::get_modified_time(&file_data));
        Ok(metadata)
    }
}

#[cfg(test)]
mod tests {
    use testspace::{TestSpace, TestSpaceFile};
    use crate::working::WorkingDirectory;
    use crate::hash::Hash;
    use std::collections::HashMap;

    #[test]
    fn create_metadata_list_test() {
        let mut ts = TestSpace::new();
        let file_list = ts.create_random_files(2, 4096);
        // TODO: Finish test
        let metadata = WorkingDirectory::create_metadata_list(file_list).expect("Failed to create metadata list");
        // println!("Data is: {:?}", metadata);
        for file in metadata {
            assert_eq!(file.filesize(), 4096);
        }
    }

    #[test]
    fn index_directory_test() {
        let mut ts = TestSpace::new();
        let mut ts2 = ts.create_child();
        ts.create_random_files(5, 4096);
        let path_to_repository = ts2.get_path();
        let path_to_working = ts.get_path();
        let ti = WorkingDirectory::new(path_to_working);
        let mut files_found = HashMap::new();
        let mut directories = Vec::new();
        WorkingDirectory::index_directory(path_to_working, &mut files_found, &mut directories).expect("Failed to index directory");
        println!("Files found");
        assert_eq!(files_found.len(), 5);
        for data in files_found {
            assert_eq!(data.1.file_size, 4096);
        }
    }

    #[test]
    fn index_repository_test() {
        let mut ts = TestSpace::new();
        let mut ts2 = ts.create_child();
        // Create fake repository files
        ts2.create_random_files(5, 4096);
        let mut ts3 = ts.create_child();
        ts3.create_random_files(4, 4096);
        ts.create_random_files(3, 4096);
        let path_to_repository = ts2.get_path();
        let path_to_working = ts.get_path();
        let ti = WorkingDirectory::new(path_to_working);
        let files = ti.index_repository().expect("Failed to index repository");
        println!("Files found");
        assert_eq!(files.len(), 12);
        for data in files {
            assert_eq!(data.1.file_size, 4096);
        }
    }

    #[test]
    fn get_changed_files_test() {
        use rand::prelude::*;
        use std::path;
        let mut rng = thread_rng();
        let mut ts = TestSpace::new();
        let original_files = ts.create_random_files(6, 4096);
        let path_to_working = ts.get_path();
        let ti = WorkingDirectory::new(path_to_working);
        let working_state = WorkingDirectory::create_metadata_list(original_files.clone()).expect("Failed to process original files");
        // Change some of the files
        // Generate a list of files to change from the list of files
        let mut files_to_change: Vec<&path::Path> = original_files.choose_multiple(&mut rng, 3).map(|x| x.as_path()).collect();
        // Change the files in the new list
        for file_to_change in &files_to_change {
            TestSpaceFile::from(*file_to_change).write_random_bytes(2048);
        }
        let mut result = ti.get_changed_files(working_state.as_slice());
        // Check that the returned files match the ones we changed
        println!("List of files: {:?}", original_files.as_slice());
        println!("List of files changed: {:?}", files_to_change.as_slice());
        println!("Detected files that were changed: {:?}", result.as_slice());
        // Sort both of the lists as the order is not guarenteed to be the same
        result.sort();
        files_to_change.sort();
        assert_eq!(result, files_to_change);
    }

    #[test]
    fn get_changed_files_since_snapshot_test() {
        unimplemented!("Test not done");
        // Take a snapshot
        // Change some files
        // Check what changed with what was changed
    }
}