use std::path;
use std::io::{self, Seek, SeekFrom};
use std::fs;
use std::convert::TryFrom;
use byteorder::{WriteBytesExt, ReadBytesExt};
use byteorder::LittleEndian;
use crate::error::{Error, UnderlyingError};
use crate::snapshots::types::Snapshot;
use crate::storage::RepositoryStorage;
use crate::storage::stream::{ReadEggExt, WriteEggExt};

type Result<T> = std::result::Result<T, Error>;

pub trait Storable {
    fn write_to_repository(&self, path_to_file: &path::Path, path_to_working: &path::Path, path_to_repository: &path::Path) -> Result<()>;
}

impl Storable for Snapshot {
    fn write_to_repository(&self, path_to_file: &path::Path, path_to_working: &path::Path, path_to_repository: &path::Path) -> Result<()> {
        println!("Storing a value");
        // Open the new snapshot
        let file_to_update = match fs::OpenOptions::new().create(true).truncate(true).write(true).open(path_to_file) {
        Ok(file) => file,
        Err(error) => return Err(Error::file_error(Some(UnderlyingError::from(error)))
            .add_debug_message(format!("Failed to create a new file (or possibly overwrite an existing one) when writing a snapshot, path was {}", path_to_file.display()))
            .add_user_message(format!("Could not create a new file, the path was {}", path_to_file.display()))),
        };
        println!("Opened File");
        let mut snapshot_writer = io::BufWriter::new(file_to_update);
        //Write the version of the snapshot
        if let Err(error) = snapshot_writer.write_u16::<byteorder::LittleEndian>(RepositoryStorage::SNAPSHOT_VERSION) {
            return Err(Error::write_error(Some(UnderlyingError::from(error)))
                .add_debug_message("Failed to write the version number of the snapshot"));
        }
        // Write the hash to the snapshot
        if let Err(error) = snapshot_writer.write_hash(self.get_hash()) {
            error.add_debug_message("Error occurred when writing a snapshot");
        }
        // Write snapshot message/description
        if let Err(error) = snapshot_writer.write_string(self.get_message()) {
            return Err(error.add_debug_message("Error occurred when writing a snapshot"))
        }
        // Write parent snapshot hash
        if let Err(error) = snapshot_writer.write_optional_hash(self.get_parent()) {
            return Err(error.add_debug_message("Error occurred when writing a snapshot"));
        }
        println!("Processing Children");
        let children = self.get_children();
        let child_count = match u16::try_from(children.len()) {
            Ok(child_count) => child_count,
            Err(error) => return Err(Error::parsing_error(Some(UnderlyingError::from(error)))
                .add_debug_message(format!("The number of children associated with the snapshot being written exceeds {}, the total number of children was {}", std::u16::MAX, children.len()))),
        };
        // TODO: Write the date/time that the snapshot was taken
        //Write number of children
        if let Err(error) = snapshot_writer.write_u16::<byteorder::LittleEndian>(child_count) {
            if let Ok(position) = snapshot_writer.seek(SeekFrom::Current(0)) {
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
                if let Ok(position) = snapshot_writer.seek(SeekFrom::Current(0)) {
                    return Err(error.add_debug_message(format!("Error occurred when writing the hash of a child snapshot, position of the writer was {}", position)));
                } else {
                    return Err(error.add_debug_message(format!("Error occurred when writing the hash of a child snapshot, position of the writer was unavailable")));
                }
            }
        }
        let snapshot_count = match u32::try_from(self.get_files().len()) {
        Ok(snapshot_count) => snapshot_count,
        Err(error) => return Err(Error::parsing_error(Some(UnderlyingError::from(error)))
            .add_debug_message(format!("The number of files stored in the snapshot exceeded the maximium of {}, it had {} files associated with it", 
                std::u32::MAX,self.get_files().len()))),
        };
        //Write number of files in the snapshot
        if let Err(error) = snapshot_writer.write_u32::<byteorder::LittleEndian>(snapshot_count) {
        return Err(Error::parsing_error(Some(UnderlyingError::from(error)))
            .add_debug_message(format!("Failed to write the number of files stored with this snapshot")));
        }
        // Write the metadata for each file in this snapshot
        for file_being_snapshot in self.get_files().iter() {
            // Write hash
            if let Err(error) = snapshot_writer.write_hash(&file_being_snapshot.hash()) {
                if let Ok(position) = snapshot_writer.seek(SeekFrom::Current(0)) {
                    return Err(error.add_debug_message(format!("Failed to write the hash of a file stored in a snapshot, writer position was {}", position)));
                } else {
                    return Err(error.add_debug_message(format!("Failed to parse the snapshot hash, writer position was not available")));
                }
            }
            // Write relative path
            if let Err(error) = snapshot_writer.write_path(file_being_snapshot.path(), path_to_working) {
                if let Ok(position) = snapshot_writer.seek(SeekFrom::Current(0)) {
                    return Err(error.add_debug_message(format!("Failed to write the path of a file stored in a snapshot, writer position was {}", position)));
                } else {
                    return Err(error.add_debug_message(format!("Failed to write the path of a file stored in a snapshot, writer position was unavailable")));
                }
            }
            // Write the file size
            if let Err(error) = snapshot_writer.write_u64::<LittleEndian>(file_being_snapshot.filesize()) {
                if let Ok(position) = snapshot_writer.seek(SeekFrom::Current(0)) {
                    return Err(Error::file_error(Some(UnderlyingError::Io(error)))
                        .add_debug_message(format!("Failed to write the filesize of a file stored in a snapshot, writer position was {}", position)));
                } else {
                    return Err(Error::file_error(Some(UnderlyingError::Io(error)))
                        .add_debug_message(format!("Failed to write the filesize of a file stored in a snapshot, writer position was unavailable")));
                }
            }
            // Write the modified time
            if let Err(error) = snapshot_writer.write_u128::<LittleEndian>(file_being_snapshot.modified_time()) {
                if let Ok(position) = snapshot_writer.seek(SeekFrom::Current(0)) {
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
}

impl Storable for &Snapshot {
    fn write_to_repository(&self, path_to_file: &path::Path, path_to_working: &path::Path, path_to_repository: &path::Path) -> Result<()> {
        println!("Storing a reference");
        (**self).write_to_repository(path_to_file, path_to_working, path_to_repository)
    }
}

impl Storable for &mut Snapshot {
    fn write_to_repository(&self, path_to_file: &path::Path, path_to_working: &path::Path, path_to_repository: &path::Path) -> Result<()> { 
        println!("Storing a mutable reference");
        (**self).write_to_repository(path_to_file, path_to_working, path_to_repository)
    }
}