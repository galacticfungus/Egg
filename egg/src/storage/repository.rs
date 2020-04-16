use std::path;
use std::io::{self, Seek, SeekFrom};
use std::fs;
use std::convert::TryFrom;
use crate::hash;
use byteorder::{self, ReadBytesExt, WriteBytesExt, LittleEndian};
use crate::storage::stream::{ReadEggExt,WriteEggExt};
use crate::snapshots::types::{Snapshot, FileMetadata};
use crate::error::{Error, UnderlyingError};
use crate::storage::Storable;

type Result<T> = std::result::Result<T, Error>;

/// Responsible for reading and writing snapshots to the repository
#[derive(Debug)]
pub struct RepositoryStorage;

impl RepositoryStorage {
    pub const SNAPSHOT_VERSION: u16 = 1;

    pub fn store(item_to_store: impl Storable, path_to_file: &path::Path, path_to_working: &path::Path, path_to_repository: &path::Path) -> Result<()> {
        item_to_store.write_to_repository(path_to_file, path_to_working, path_to_repository)
    }

    pub fn restore_snapshot(path_to_snapshot: &path::Path, path_to_working: &path::Path) -> Result<Snapshot> {
        let file = match fs::OpenOptions::new().read(true).open(path_to_snapshot) {
            Ok(snapshot_file) => snapshot_file,
            Err(error) => {
                if path_to_snapshot.exists() {
                    return Err(Error::file_error(Some(UnderlyingError::from(error)))
                        .add_debug_message(format!("Failed to open the snapshot metadata file when trying to read a snapshot, the path was {} and did exist", path_to_snapshot.display()))
                        .add_user_message(format!("Failed to open a file when trying to read a snapshot, the path was {} and does exist", path_to_snapshot.display())));
                }
                else {
                    return Err(Error::file_error(Some(UnderlyingError::from(error)))
                        .add_debug_message(format!("Failed to open the snapshot file when trying to read a snapshot, the path {} didn't exist", path_to_snapshot.display()))
                        .add_user_message(format!("Failed to open a file when trying to read a snapshot, the path was {} and that path doesn't exist", path_to_snapshot.display())));
                }
            },
        };
        let mut snapshot_reader = io::BufReader::new(file);

        // Read version of data file
        let data_version = match snapshot_reader.read_u16::<byteorder::LittleEndian>() {
            Ok(version) => version,
            Err(error) => return Err(Error::parsing_error(Some(UnderlyingError::from(error)))
                .add_debug_message("Failed to read the version of a snapshot")),
        };
        // Ensure we can load this snapshot version
        Self::check_snapshot_version(data_version);

        // Read the Snapshot ID hash
        let id_hash = match snapshot_reader.read_hash() {
            Ok(hash) => hash,
            Err(error) => {
                if let Ok(position) = snapshot_reader.seek(SeekFrom::Current(0)) {
                    return Err(error.add_debug_message(format!("Failed to parse the snapshot hash, file was {} and reader position was {}", path_to_snapshot.display(), position)));
                } else {
                    return Err(error.add_debug_message(format!("Failed to parse the snapshot hash, file was {} and reader position was not available", path_to_snapshot.display())));
                }
            }
        };
        // Read snapshot message/description
        let message_string = match snapshot_reader.read_string() {
            Ok(message_string) => message_string,
            Err(error) => {
                if let Ok(position) = snapshot_reader.seek(SeekFrom::Current(0)) {
                    return Err(error.add_debug_message(format!("Failed to parse the snapshot message, path was {} and the reader position was {}", path_to_snapshot.display(), position))
                                    .add_user_message(format!("A file in the repository may be corrupt, failed to read a string when parsing a snapshot file, path of the file was {}", path_to_snapshot.display())));
                } else {
                    return Err(error.add_debug_message(format!("Failed to parse the snapshot message, reader position was not available"))
                                    .add_user_message(format!("A file in the repository may be corrupt, failed to read a string when parsing a snapshot file, apth of the file was {}", path_to_snapshot.display())));
                }          
            },
        };
        // Read parent
        let parent_hash = match snapshot_reader.read_optional_hash() {
            Ok(parent_hash) => parent_hash,
            Err(error) => {
                if let Ok(position) = snapshot_reader.seek(SeekFrom::Current(0)) {
                    return Err(error.add_debug_message(format!("Failed to parse the snapshot parent hash, reader position was {}", position)));
                } else {
                    return Err(error.add_debug_message(format!("Failed to parse the snapshot parent hash, reader position was not available")));
                }
            },
        };

        //Read number of children
        let child_count = match snapshot_reader.read_u16::<byteorder::LittleEndian>() {
            Ok(child_count) => child_count,
            Err(error) => return Err(Error::parsing_error(Some(UnderlyingError::from(error)))
                .add_debug_message("Failed to read the number of child snapshots")),
        };
        let mut children = Vec::with_capacity(usize::from(child_count));
        // Read the hash of each child
        for _ in (0..child_count).into_iter() {
            let child_hash = match snapshot_reader.read_hash() {
                Ok(child_hash) => child_hash,
                Err(error) => {
                    return Err(error.add_debug_message("Failed to read a hash of a snapshot child"));
                },
            };
            children.push(child_hash);
        }
        //Read number of files in the snapshot
        let files_in_snapshot = match snapshot_reader.read_u32::<byteorder::LittleEndian>() {
            Ok(files_in_snapshot) => files_in_snapshot,
            Err(error) => return Err(Error::parsing_error(Some(UnderlyingError::from(error)))
                .add_debug_message("Failed to read the number of files in a snapshot")),
        };
        let number_of_files_to_read = match usize::try_from(files_in_snapshot) {
            Ok(number_of_files_to_read) => number_of_files_to_read,
            Err(error) => return Err(Error::parsing_error(Some(UnderlyingError::from(error)))
                .add_debug_message("Failed to read the number of child snapshots")),
        };
        let mut file_list = Vec::with_capacity(number_of_files_to_read);
        // Read the hash and path, filesize and modification time for each file in this snapshot
        for _ in 0..files_in_snapshot {
            let metadata = match RepositoryStorage::read_file_metadata(path_to_working, &id_hash, &mut snapshot_reader) {
                Ok(metadata) => metadata,
                Err(error) => return Err(error),
                Err(error) => {
                    return Err(error.add_debug_message(format!("Could not restore the metadata for a file stored in snapshot {}", id_hash)));
                },
            };
            file_list.push(metadata);
        }
        let snapshot = Snapshot::new(id_hash, message_string, file_list, children, parent_hash);
        Ok(snapshot)
    }

    fn read_file_metadata(path_to_working: &path::Path, snapshot_hash: &hash::Hash, snapshot_reader: &mut io::BufReader<fs::File>) -> Result<FileMetadata> {
        // Read hash
        let hash = match snapshot_reader.read_hash() {
            Err(error) => {
                return Err(error.add_debug_message(format!("Error while reading the hash of a file stored as part of a snapshot {}", snapshot_hash)));
            },
            Ok(hash) => hash,
        };
        // Read path using the working path as the base path
        let path_to_file= match snapshot_reader.read_path(path_to_working) {
            Ok(path_to_file) => path_to_file,
            Err(error) => {
                return Err(error.add_debug_message(format!("Failed to read the path of a file stored as part of a snapshot {}", snapshot_hash)));
            },
        };
        // Read file size
        let file_size = match snapshot_reader.read_u64::<LittleEndian>() {
            Ok(file_size) => file_size,
            Err(error) => {
                let error = Error::parsing_error(Some(UnderlyingError::Io(error)))
                    .add_debug_message(format!("Failed to read the file size of a file stored as part of a snapshot, {}", snapshot_hash))
                    .add_user_message("Failed to read a snapshot correctly, metadata about files stored in the snapshot could not be read");
                return Err(error);
            }
        };
        // Read modified time
        let modified_time = match snapshot_reader.read_u128::<LittleEndian>() {
            Ok(modified_time) => modified_time,
            Err(error) => {
                let error = Error::parsing_error(Some(UnderlyingError::Io(error)))
                    .add_debug_message(format!("Failed to read the modified time of a file stored as part of a snapshot, {}", snapshot_hash))
                    .add_user_message("Failed to read a snapshot correctly, metadata about files stored in the snapshot could not be read");
                return Err(error);
            }
        };
        Ok(FileMetadata::new(hash, file_size, path_to_file, modified_time))
    }

    // Check that the snapshot file version matches the version that egg considers current
    // If it is lower then upgrade the file
    // If it is higher then exit as a newer version of egg is needed to read the repository
    fn check_snapshot_version(version_read: u16) {
        if version_read != RepositoryStorage::SNAPSHOT_VERSION {
            //Upgrade path
            unimplemented!("Snapshot data file upgrade path not implemented");
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::snapshots::types::{Snapshot};
    use testspace::TestSpace;
    use crate::storage::RepositoryStorage;
    use crate::hash::Hash;
    use crate::storage::LocalFileStorage;
    use crate::working::WorkingDirectory;

    #[test]
    fn test_read_write_snapshot() {
        // Test writing and reading a snapshot
        let mut ts = TestSpace::new();
        let ts2 = ts.create_child();
        // Create files to snapshot
        let file_list = ts.create_random_files(3, 2048);
        // Path to test work directory and repository
        let working_path = ts.get_path();
        let repository_path = ts2.get_path();
        // TODO: Rename function
        let hashed_files = WorkingDirectory::create_metadata_list(file_list).expect("Failed to create metadata file list");
        let id = Hash::generate_random_hash();
        let snapshot = Snapshot::new(id.clone(), String::from("message"), hashed_files, Vec::new(), None);
        let path_to_snapshot = repository_path.join(String::from(id.clone()));
        RepositoryStorage::store(snapshot, path_to_snapshot.as_path(), working_path, repository_path).expect("Failed writing snapshot");
        let file_name = String::from(&id);
        let path_to_snapshot = repository_path.join(file_name);
        let result = RepositoryStorage::restore_snapshot(path_to_snapshot.as_path(), working_path).expect("Failed reading snapshot");
        assert_eq!("message", result.get_message());
        assert_eq!(&id, result.get_hash());
    }

    //
    #[test]
    fn test_snapshot_id() {
        use rand::seq::SliceRandom;
        // Tests that a snapshot id does not vary by the order of the files being snapshotted
        let mut ts = TestSpace::new();
        // Create files to snapshot
        let file_list = ts.create_random_files(3, 2048);
        let mut hashed_files = WorkingDirectory::create_metadata_list(file_list).expect("Failed to process list of files to stage");
        // Shuffle file list ensuring that it is not equal to original order
        let mut shuffled_list = hashed_files.clone();
        let mut rng = rand::thread_rng();
        shuffled_list.shuffle(&mut rng);
        // BUG: This test can fail randomly if the list is shuffled to the same order
        assert_ne!(hashed_files, shuffled_list);
        // Unshuffled list should produce same hash id as shuffled list
        let first_id = Hash::generate_snapshot_id("message", hashed_files.as_mut_slice());
        let second_id = Hash::generate_snapshot_id("message", shuffled_list.as_mut_slice());
        assert_eq!(first_id, second_id);
    }

    #[test]
    fn test_snapshot_with_parent() {
        // Tests that it is possible to create a hierarchy of snapshots, a parent and a child
        let mut ts = TestSpace::new().allow_cleanup(true);
        let ts2 = ts.create_child();
        let file_list = ts.create_random_files(2, 2048);
        let file_list2 = ts.create_random_files(3, 4096);
        let path_to_working = ts.get_path();
        let path_to_repository = ts2.get_path();
        let mut hashed_files = WorkingDirectory::create_metadata_list(file_list).expect("Failed to process list of files");
        let mut hashed_files2 = WorkingDirectory::create_metadata_list(file_list2).expect("Failed to process second file list");
        let parent_id = Hash::generate_snapshot_id("message", hashed_files.as_mut_slice());
        let mut parent = Snapshot::new(parent_id.clone(), String::from("message"), hashed_files, Vec::new(), None);
        let child_id = Hash::generate_snapshot_id("diddly", hashed_files2.as_mut_slice());
        let child = Snapshot::new(child_id.clone(), String::from("diddly"), hashed_files2, Vec::new(), Some(parent_id));
        parent.add_child(child_id.clone());
        // file_storage is responsible for actually storing user data locally
        let file_storage = LocalFileStorage::initialize(path_to_repository).expect("Failed to initialize local storage");
        let path_to_parent_snapshot = path_to_repository.join(String::from(parent.get_hash()));
        let path_to_child_snapshot = path_to_repository.join(String::from(child.get_hash()));
        
        RepositoryStorage::store(parent, path_to_parent_snapshot.as_path(), path_to_working, path_to_working).expect("Failed to write parent snapshot");
        RepositoryStorage::store(child, path_to_child_snapshot.as_path(), path_to_working, path_to_working).expect("Failed to write child snapshot");

        let parent_result = RepositoryStorage::restore_snapshot(path_to_parent_snapshot.as_path(), path_to_working).expect("Failed to read parent snapshot");
        let child_result = RepositoryStorage::restore_snapshot(path_to_child_snapshot.as_path(), path_to_working).expect("Failed to read parent snapshot");
        // Check that the child has a correct parent
        assert_eq!(child_result.get_parent().unwrap(), parent_result.get_hash());
        let parent_child = &parent_result.get_children()[0];
        // Check that the parent has a correct child
        assert_eq!(parent_child, child_result.get_hash());
    }

    // TODO: Test reading snapshot with children
}