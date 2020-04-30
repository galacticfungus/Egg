use std::path;
use std::fs;
use std::io::{self, Seek};
use std::convert::TryFrom;

use crate::{error::Error, error::UnderlyingError, hash};
use super::{types::Snapshot, RepositorySnapshots, FileMetadata, SnapshotId, SnapshotLocation};
use crate::storage::stream::{WriteEggExt, ReadEggExt};

use byteorder::{ReadBytesExt, WriteBytesExt};

impl RepositorySnapshots {
    pub const SNAPSHOT_VERSION: u16 = 1;
    // TODO: This needs to incorporate the snapshots location into its logic, ie if the snapshot was stored in a packed file that packed file needs to be rewritten
    // pub fn write_snapshot(&self, snapshot_to_write: &Snapshot, location: SnapshotLocation) -> Result<(), Error> {
    //     match location {
    //         SnapshotLocation::Simple => {
    //             let path_to_file = self.path_to_repository.join("snapshots").join(snapshot_to_write.get_hash().to_string());
    //             self.write_simple_snapshot(snapshot_to_write, path_to_file.as_path())?;
    //         }
    //     }
    //     Ok(())
    // }

    // TODO: This must take a random path and just write to the file to be compatible with the atomic system

    pub fn write_simple_snapshot(snapshot_to_write: &Snapshot, path_to_file: &path::Path, path_to_working: &path::Path) -> Result<(), Error> {
        
        let file_to_update = match fs::OpenOptions::new().create(true).write(true).open(path_to_file) {
            Ok(file) => file,
            Err(error) => return Err(Error::file_error(Some(UnderlyingError::from(error)))
                .add_debug_message(format!("Failed to create a new file when writing a snapshot, path was {}", path_to_file.display()))
                .add_user_message(format!("Could not create a new file, the path was {}", path_to_file.display()))),
        };
        let mut snapshot_writer = io::BufWriter::new(file_to_update);
        //Write the version of the snapshot
        if let Err(error) = snapshot_writer.write_u16::<byteorder::LittleEndian>(Self::SNAPSHOT_VERSION) {
            return Err(Error::write_error(Some(UnderlyingError::from(error)))
                .add_debug_message("Failed to write the version number of the snapshot"));
        }
        // Write the hash to the snapshot
        if let Err(error) = snapshot_writer.write_hash(snapshot_to_write.get_hash()) {
            error.add_debug_message("Error occurred when writing a snapshot");
        }
        // Write snapshot message/description
        if let Err(error) = snapshot_writer.write_string(snapshot_to_write.get_message()) {
            return Err(error.add_debug_message("Error occurred when writing a snapshot"))
        }
        // Write parent snapshot hash
        if let Err(error) = snapshot_writer.write_optional_hash(snapshot_to_write.get_parent()) {
            return Err(error.add_debug_message("Error occurred when writing a snapshot"));
        }
        let children = snapshot_to_write.get_children();
        let child_count = match u16::try_from(children.len()) {
            Ok(child_count) => child_count,
            Err(error) => return Err(Error::parsing_error(Some(UnderlyingError::from(error)))
                .add_debug_message(format!("The number of children associated with the snapshot being written exceeds {}, the total number of children was {}", std::u16::MAX, children.len()))),
        };
        // TODO: Write the date/time that the snapshot was taken
        //Write number of children
        if let Err(error) = snapshot_writer.write_u16::<byteorder::LittleEndian>(child_count) {
            if let Ok(position) = snapshot_writer.seek(io::SeekFrom::Current(0)) {
                return Err(Error::parsing_error(Some(UnderlyingError::from(error)))
                    .add_debug_message(format!("Failed to write the number of children associated with the snapshot being written, writer position was {}", position)));
            } else {
                return Err(Error::parsing_error(Some(UnderlyingError::from(error)))
                    .add_debug_message(format!("Failed to write the number of children associated with the snapshot being written, writer position was unavailable")));
            }
        }
        // Write the hash of each child
        for child in children {
            if let Err(error) = snapshot_writer.write_hash(child) {
                if let Ok(position) = snapshot_writer.seek(io::SeekFrom::Current(0)) {
                    return Err(error.add_debug_message(format!("Error occurred when writing the hash of a child snapshot, position of the writer was {}", position)));
                } else {
                    return Err(error.add_debug_message(format!("Error occurred when writing the hash of a child snapshot, position of the writer was unavailable")));
                }
            }
        }
        let snapshot_count = match u32::try_from(snapshot_to_write.get_files().len()) {
        Ok(snapshot_count) => snapshot_count,
        Err(error) => return Err(Error::parsing_error(Some(UnderlyingError::from(error)))
            .add_debug_message(format!("The number of files stored in the snapshot exceeded the maximium of {}, it had {} files associated with it", 
                std::u32::MAX, snapshot_to_write.get_files().len()))),
        };
        //Write number of files in the snapshot
        if let Err(error) = snapshot_writer.write_u32::<byteorder::LittleEndian>(snapshot_count) {
        return Err(Error::parsing_error(Some(UnderlyingError::from(error)))
            .add_debug_message(format!("Failed to write the number of files stored with this snapshot")));
        }
        // Write the metadata for each file in this snapshot
        for file_being_snapshot in snapshot_to_write.get_files().iter() {
            // Write hash
            if let Err(error) = snapshot_writer.write_hash(&file_being_snapshot.hash()) {
                if let Ok(position) = snapshot_writer.seek(io::SeekFrom::Current(0)) {
                    return Err(error.add_debug_message(format!("Failed to write the hash of a file stored in a snapshot, writer position was {}", position)));
                } else {
                    return Err(error.add_debug_message(format!("Failed to parse the snapshot hash, writer position was not available")));
                }
            }
            // Write relative path
            if let Err(error) = snapshot_writer.write_path(file_being_snapshot.path(), path_to_working) {
                if let Ok(position) = snapshot_writer.seek(io::SeekFrom::Current(0)) {
                    return Err(error.add_debug_message(format!("Failed to write the path of a file stored in a snapshot, writer position was {}", position)));
                } else {
                    return Err(error.add_debug_message(format!("Failed to write the path of a file stored in a snapshot, writer position was unavailable")));
                }
            }
            // Write the file size
            if let Err(error) = snapshot_writer.write_u64::<byteorder::LittleEndian>(file_being_snapshot.filesize()) {
                if let Ok(position) = snapshot_writer.seek(io::SeekFrom::Current(0)) {
                    return Err(Error::file_error(Some(UnderlyingError::Io(error)))
                        .add_debug_message(format!("Failed to write the filesize of a file stored in a snapshot, writer position was {}", position)));
                } else {
                    return Err(Error::file_error(Some(UnderlyingError::Io(error)))
                        .add_debug_message(format!("Failed to write the filesize of a file stored in a snapshot, writer position was unavailable")));
                }
            }
            // Write the modified time
            if let Err(error) = snapshot_writer.write_u128::<byteorder::LittleEndian>(file_being_snapshot.modified_time()) {
                if let Ok(position) = snapshot_writer.seek(io::SeekFrom::Current(0)) {
                    return Err(Error::file_error(Some(UnderlyingError::Io(error)))
                        .add_debug_message(format!("Failed to write the time last modified of a file stored in a snapshot, writer position was {}", position)));
                } else {
                    return Err(Error::file_error(Some(UnderlyingError::Io(error)))
                        .add_debug_message(format!("Failed to write the time last modified of a file stored in a snapshot, writer position was unavailable")));
                }
            }
        }
        Ok(())
    }

    fn read_file_metadata(path_to_working: &path::Path, snapshot_hash: &hash::Hash, snapshot_reader: &mut io::BufReader<fs::File>) -> Result<FileMetadata, Error> {
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
        let file_size = match snapshot_reader.read_u64::<byteorder::LittleEndian>() {
            Ok(file_size) => file_size,
            Err(error) => {
                let error = Error::parsing_error(Some(UnderlyingError::Io(error)))
                    .add_debug_message(format!("Failed to read the file size of a file stored as part of a snapshot, {}", snapshot_hash))
                    .add_user_message("Failed to read a snapshot correctly, metadata about files stored in the snapshot could not be read");
                return Err(error);
            }
        };
        // Read modified time
        let modified_time = match snapshot_reader.read_u128::<byteorder::LittleEndian>() {
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
        if version_read != Self::SNAPSHOT_VERSION {
            //Upgrade path
            unimplemented!("Snapshot data file upgrade path not implemented");
        }
    }

    fn load_simple_snapshot(hash: &hash::Hash, path_to_repository: &path::Path, path_to_working: &path::Path) -> Result<Snapshot, Error> {
        let file_name = String::from(hash);
        
        let path_to_snapshot = path_to_repository.join(Self::get_path()).join(file_name);
        let file = match fs::OpenOptions::new().read(true).open(path_to_snapshot.as_path()) {
            Ok(snapshot_file) => snapshot_file,
            Err(error) => {
                if path_to_snapshot.exists() {
                    return Err(Error::file_error(Some(UnderlyingError::from(error)))
                        .add_debug_message(format!("Failed to open the snapshot metadata file when trying to read a snapshot, the path was {} and did exist", path_to_snapshot.display()))
                        .add_user_message(format!("Failed to open a file when trying to read a snapshot, the path was {} and does exist", path_to_snapshot.display())));
                } else {
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
        let snapshot_hash = match snapshot_reader.read_hash() {
            Ok(hash) => hash,
            Err(error) => {
                if let Ok(position) = snapshot_reader.seek(io::SeekFrom::Current(0)) {
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
                if let Ok(position) = snapshot_reader.seek(io::SeekFrom::Current(0)) {
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
                if let Ok(position) = snapshot_reader.seek(io::SeekFrom::Current(0)) {
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
        // This implies that each
        for _ in 0..files_in_snapshot {
            // TODO: This call to read_file_metadata seems incorrect
            let metadata = match Self::read_file_metadata(path_to_working, &snapshot_hash, &mut snapshot_reader) {
                Ok(metadata) => metadata,
                Err(error) => return Err(error.add_debug_message(format!("Could not restore the metadata for a file stored in snapshot {}", snapshot_hash))),
            };
            file_list.push(metadata);
        }
        let snapshot = Snapshot::new(snapshot_hash, message_string, file_list, children, parent_hash);
        Ok(snapshot)
    }

    /// Loads a snapshot from storage, the appropriate method for loading the snapshot is given by the location
    /// it returns the index of the new snapshot
    pub fn load_snapshot(&mut self, hash: &hash::Hash, location: SnapshotLocation, path_to_repository: &path::Path, path_to_working: &path::Path) -> Result<usize, Error> {
        let snapshot = match location {
            SnapshotLocation::Simple => Self::load_simple_snapshot(&hash, path_to_repository, path_to_working)
                .map_err(|err| err.add_generic_message("While loading a simple snapshot"))?,
            _ => panic!("Non simple snapshot located: {:?}", location),
        };
        
        if let Some(parent_hash) = snapshot.get_parent() {
            // If the snapshot we are loading has a parent, we add the parents hash to the index unless the parent is already indexed
            match self.index.get(&parent_hash) {
                // Adding a hash to the index that is already present will not cause an error, the hash would just need to be relocated again
                None => {
                    self.index.insert(parent_hash.clone(), SnapshotId::NotLocated(parent_hash.clone()));
                },
                _ => (), // Any other index entry requires no modification as loading the parent hash is not required
            }
        }
        // Add Child ID's to the index
        for child_hash in snapshot.get_children() {
            match self.index.get(&child_hash) {
                // Adding a hash to the index that is already in there does nothing but we need to ensure that entry is not marked Located or Indexed
                None => {
                    self.index.insert(child_hash.clone(), SnapshotId::NotLocated(child_hash.clone()));
                },
                _ => (), // Any other index entry requires no modification as loading the child snapshot is not required
            }
        }
        
        // If an index is already associated with this hash panic as its a bug and should be impossible
        debug_assert!(matches!(self.index.get(&hash), Some(&SnapshotId::Indexed(_, _))) == false);

        // Add the snapshot that was loaded to the index replacing any previous entry, marking it as Indexed and supplying its index in the vector
        self.index.insert(hash.clone(), SnapshotId::Indexed(self.snapshots.len(), hash.clone()));
        self.snapshots.push(snapshot);
        Ok(self.snapshots.len() - 1)
    }

    
}
#[cfg(test)]
mod tests {
    use testspace::TestSpace;
    use super::RepositorySnapshots;
    use crate::snapshots::types::Snapshot;
    use crate::hash::Hash;
    use crate::working::WorkingDirectory;

    #[test]
    fn write_parse_snapshot_test() {
        let mut ts = TestSpace::new();
        let mut ts2 = ts.create_child();
        let file_list = ts.create_random_files(4, 4096);
        ts2.create_dir("snapshots");
        
        let path_to_working = ts.get_path();
        let path_to_repository = ts2.get_path();
        
        let mut files_to_snapshot = WorkingDirectory::create_metadata_list(file_list).expect("Failed to get metadata of files to snapshot");
        let message = String::from("Test Snapshot");
        let id = Hash::generate_snapshot_id(message.as_str(), files_to_snapshot.as_mut_slice());
        let snapshot = Snapshot::new(id.clone(), message, files_to_snapshot, Vec::new(), None);
    
        {
            // We must specify the exact file to write to for this to be compatible with atomic
            // Note: .to_string and String::from are not the same
            let path_to_file = path_to_repository.join("snapshots").join(snapshot.get_hash().to_string());
            RepositorySnapshots::write_simple_snapshot(&snapshot, path_to_file.as_path(), path_to_working).expect("Failed writing snapshot");
        }
        let snapshot_result = RepositorySnapshots::load_simple_snapshot(snapshot.get_hash(), path_to_repository, path_to_working).expect("Failed reading snapshot");
        assert_eq!(snapshot, snapshot_result);
    }

}