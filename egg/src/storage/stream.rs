use crate::storage;
use byteorder::{ LittleEndian, ReadBytesExt, WriteBytesExt };
use crate::hash;
use std::io::{ self, Read, Seek, SeekFrom, Write };
use std::path;
use std::result;
use crate::error::{Error, UnderlyingError};
pub type Result<T> = result::Result<T, Error>;
use std::convert::TryFrom;

//todo: benchmark storing paths as u16 and u32
//todo: add these functions as extensions of readers and writers
/// Reads a path from the reader, it first reads the path length and then reads the path bytes
pub trait ReadEggExt: io::Read + ReadBytesExt + io::Seek {
    fn read_string(&mut self) -> Result<String> {
        let string_length = match self.read_u16::<byteorder::LittleEndian>() {
            Ok(string_length) => string_length,
            Err(error) => return Err(Error::parsing_error(Some(UnderlyingError::from(error)))
                .add_debug_message("Failed to read the length of a string with the Read trait extensions")),
        };
        // let mut byte_data = Vec::with_capacity(string_length as usize);
        let mut string_reader = self.take(string_length as u64);
        let mut string_from_bytes = String::with_capacity(usize::from(string_length));
        if let Err(error) = string_reader.read_to_string(&mut string_from_bytes) {
            
            return Err(Error::parsing_error(Some(UnderlyingError::from(error)))
                .add_debug_message("Failed to read a string with Read Trait extensions"));
        }
        Ok(string_from_bytes)
    }

    fn read_optional_hash(&mut self) -> Result<Option<hash::Hash>> {
        // Read hash length
        // TODO: This can at least be changed to a byte since it really only represents the presence of the hash
        let hash_length = match self.read_u16::<LittleEndian>() {
            Ok(hash_length) if hash_length == 0 => return Ok(None),
            Ok(hash_length) => hash_length,
            Err(error) => return Err(Error::parsing_error(Some(UnderlyingError::from(error)))
                .add_debug_message("Failed to read the length of an optional hash with the Read trait extensions")),
        };
        // Only blake 2 hashes are supported
        debug_assert_eq!(hash_length, 64, "Hash being written was not 64 bytes long");
        // Read hash
        Ok(Some(self.read_hash()?))
    }

    fn read_hash(&mut self) -> Result<hash::Hash> {
        // TODO: Use a constant for both the write and read method instead of 64
        let mut buffer = Vec::with_capacity(64);
        let mut hash_buffer = self.take(64);
        if let Err(error) = hash_buffer.read_to_end(&mut buffer) {
            return Err(Error::parsing_error(Some(UnderlyingError::from(error)))
                .add_debug_message("Failed to read a hash with the Read trait extensions"));
        }
        Ok(hash::Hash::from(buffer))
    }

    fn read_path(&mut self, relative_to: &path::Path) -> Result<path::PathBuf> {
        //length of path
        let path_length = match self.read_u16::<LittleEndian>() {
            Ok(path_length) => path_length,
            Err(error) => {
                return Err(Error::parsing_error(Some(UnderlyingError::from(error)))
                    .add_debug_message("Failed to read the length of a path with the Read trait extensions"));
            }
        };
        //read path bytes
        let mut buffer = Vec::with_capacity(usize::from(path_length));
        let path_size = u64::from(path_length);
        let mut path_buffer = self.take(path_size);
        if let Err(error) = path_buffer.read_to_end(&mut buffer) {
            return Err(Error::parsing_error(Some(UnderlyingError::from(error)))
                .add_debug_message("Failed to read a path with the Read trait extensions"));
        }

        let path_string = match String::from_utf8(buffer) {
            Ok(path_string) => path_string,
            Err(error) => {
                return Err(Error::parsing_error(Some(UnderlyingError::from(error)))
                    .add_debug_message("Failed to read the length of a string with the Read trait extensions"));
            }
        };
        let relative_path = path::PathBuf::from(path_string);
        let final_path = relative_to.join(relative_path);
        debug_assert_eq!(
            final_path.is_absolute(),
            true,
            "BUG: Path being read is relative: {}",
            final_path.display()
        );
        Ok(final_path)
    }

    fn read_optional_path(&mut self, relative_to: &path::Path) -> Result<Option<path::PathBuf>> {
        let length_of_path = match self.read_u16::<LittleEndian>() {
            Ok(path_length) if path_length == 0 => return Ok(None),
            Ok(path_length) => path_length,
            Err(error) => {
                return Err(Error::parsing_error(Some(UnderlyingError::from(error)))
                    .add_debug_message("Failed to read the length of an optional path with the Read trait extensions"));
            }
        };
        let mut buffer = Vec::with_capacity(usize::from(length_of_path));
        let mut path_buffer = self.take(u64::from(length_of_path));
        if let Err(error) = path_buffer.read_to_end(&mut buffer) {
            return Err(Error::parsing_error(Some(UnderlyingError::from(error)))
                .add_debug_message("Failed to read the a optional string with the Read trait extensions"));
        };
        let path_string = match String::from_utf8(buffer) {
            Ok(path_string) => path_string,
            Err(error) => return Err(Error::parsing_error(Some(UnderlyingError::from(error)))
                .add_debug_message("Failed to convert a string to a path")),
        };
        let relative_path = path::PathBuf::from(path_string);
        let final_path = relative_to.join(relative_path);
        debug_assert!(final_path.is_absolute(), "BUG: Path being read was relative: {}", final_path.display());
        Ok(Some(final_path))
    }
}

pub trait WriteEggExt: Write + WriteBytesExt + Seek {
    fn write_optional_hash(&mut self, hash_to_write: Option<&hash::Hash>) -> Result<()> {
        // Are we writing a hash or not?
        match hash_to_write {
            Some(hash_to_write) => {
                debug_assert_eq!(hash_to_write.len(), 64, "Hash being written was not 64 bytes long");
                // Write hash length of 64 of blake2 hash
                // TODO: This doesn't need to be u16
                if let Err(error) = self.write_u16::<LittleEndian>(64) {
                    // TODO: Fix error message
                    return Err(Error::write_error(Some(UnderlyingError::from(error)))
                        .add_debug_message("Failed to write length of optional (Some) hash, error was {}"));
                };
                if let Err(error) = self.write_hash(hash_to_write) {
                    return Err(error.add_debug_message("While writing an optional hash"));
                };
            },
            None => {
                // We can write a hash length of zero here
                if let Err(error) = self.write_u16::<LittleEndian>(0) {
                    // TODO: Fix Error Message
                    return Err(Error::write_error(Some(UnderlyingError::from(error)))
                        .add_debug_message(format!("Failed to write length of optional (None) hash")));
                };
            },
        };
        Ok(())
    }

    fn write_string<S: AsRef<str>>(&mut self, string_to_write: S) -> Result<()> {
        let string_to_write = string_to_write.as_ref();
        let string_length = match u16::try_from(string_to_write.len()) {
            Ok(string_length) => string_length,
            Err(error) => {
                return Err(Error::write_error(Some(UnderlyingError::from(error)))
                    .add_debug_message(format!("Failed to convert a usize into a u16 when writing a string length, actual string size was {}", string_to_write.len())));
            },
        };
        if let Err(error) = self.write_u16::<LittleEndian>(string_length) {
            return Err(Error::write_error(Some(UnderlyingError::from(error)))
                .add_debug_message(format!("Failed to write length of string, length was {}", string_length)));
        }
        // TODO: I should detect interrupted error and repeat write operation
        if let Err(error) = self.write(string_to_write.as_bytes()) {
            return Err(Error::write_error(Some(UnderlyingError::from(error)))
                .add_debug_message(format!("Failed to write a string, string was {}", string_to_write)));
        }
        Ok(())
    }

    fn write_hash(&mut self, hash_to_write: &hash::Hash) -> Result<()> {
        debug_assert_eq!(hash_to_write.len(), 64, "Hash being written was not 64 bytes long");
        if let Err(error) = self.write(hash_to_write.as_bytes()) {
            return Err(Error::write_error(Some(UnderlyingError::from(error)))
                .add_debug_message(format!("Failed to write a hash, the hash bytes were {:?}", hash_to_write)));
        }
        Ok(())
    }

    fn write_hash_at_position(&mut self, hash_to_write: &hash::Hash, position: SeekFrom) -> Result<()> {
        if let Err(error) = self.seek(position) {
            return Err(Error::write_error(Some(UnderlyingError::from(error)))
                .add_debug_message(format!("Failed to seek to the position {:?} when writing a hash", position)));
        }
        if let Err(error) = self.write_hash(hash_to_write) {
            return Err(error.add_debug_message("Occurred while writing a hash at position"));
        }
        Ok(())
    }

    fn write_u16_at_position(&mut self, value: u16, position: SeekFrom) -> Result<()> {
        if let Err(error) = self.seek(position) {
            return Err(Error::write_error(Some(UnderlyingError::from(error)))
                .add_debug_message(format!("Failed to seek to the position {:?} when writing a u16", position)));
        }
        if let Err(error) = self.write_u16::<LittleEndian>(value) {
            return Err(Error::write_error(Some(UnderlyingError::from(error)))
                .add_debug_message(format!("Failed to write a u16 {} to the position {:?}", value, position)));
        }
        Ok(())
    }

    /// Write an optional path, meaning a path that is contained in an Option, a path is written by
    /// first writing its length and then the bytes, a path length of zero denotes that there is no
    /// path
    fn write_optional_path<P: AsRef<path::Path>>(&mut self, path_to_write: Option<P>, relative_to: &path::Path) -> Result<()> {
        match path_to_write {
            None => {
                if let Err(error) = self.write_u16::<LittleEndian>(0) {
                    let error = Error::write_error(Some(UnderlyingError::from(error)))
                        .add_debug_message(format!("Failed to write the length of an optional path that was None"));
                    return Err(error);
                }
                Ok(())
            },
            Some(path_to_write) => {
                if let Err(error) = self.write_path(path_to_write, relative_to) {
                    return Err(error.add_debug_message(format!("Error occured while trying to write a path that was present but optional")));
                }
                Ok(())
            }
        }
    }

    fn write_path<P: AsRef<path::Path>>(&mut self, path_to_write: P, relative_to: &path::Path) -> Result<()> {
        // Convert path to string
        let path_to_write = path_to_write.as_ref();
        let relative_path_to_write = match path_to_write.strip_prefix(relative_to) {
            Ok(relative_path) => relative_path,
            Err(error) => { 
                let error = Error::write_error(Some(UnderlyingError::from(error)))
                    .add_debug_message(format!("In preparation for writing a path, A conversion failed when converting an absolute path {} into a path that is relative to {}", path_to_write.display(), relative_to.display()));
                return Err(error);
            }
        };
        debug_assert_eq!(relative_path_to_write.is_relative(), true, "BUG: Path being stored was absolute: {}", relative_path_to_write.display());
        //Convert path to string, need to use  a cross platform standard for paths, requiring any path to be UTF8 compatible does this
        let path_string = match relative_path_to_write.to_str() {
            Some(converted_string) => converted_string,
            None => {
                let error = Error::write_error(None)
                    .add_debug_message(format!("Failed to convert a path into a string when preparing to write a path, path was {}", relative_path_to_write.display()));
                return Err(error);
            },
        };
        //Length the of the path
        let length_of_path = match u16::try_from(path_string.len()) {
            Ok(length_of_path) => length_of_path,
            Err(error) => {
                let error = Error::write_error(Some(UnderlyingError::from(error)))
                    .add_debug_message(format!("Failed to convert a usize into a u16 when writing the length of a path, path length was {}", path_string.len()));
                return Err(error);
            }
        };
        if let Err(error) = self.write_u16::<LittleEndian>(length_of_path) {
            let error = Error::write_error(Some(UnderlyingError::from(error))).add_debug_message("Failed to write the length of a path");
            return Err(error);
        }
        if let Err(error) = self.write(path_string.as_bytes()) {
            return Err(Error::write_error(Some(UnderlyingError::from(error))).add_debug_message(format!("Failed to write the bytes making up a path")));
        }
        Ok(())
    }
}

// All types that implement Read and ReadBytesExt get ReadEggTypes
impl<R: io::Read + ReadBytesExt + Seek + ?Sized> ReadEggExt for R {}
// All types that implement Write and WriteBytesExt get WriteEggTypes
impl<W: io::Write + WriteBytesExt + Seek + ?Sized> WriteEggExt for W {}

#[cfg(test)]
mod tests {

    use super::{ReadEggExt, WriteEggExt};
    use std::io::{self, Seek, SeekFrom, Read};
    use testspace::{TestSpace};
    use byteorder;
    use crate::hash;

    #[test]
    fn test_read_write_string() {
      let ts = TestSpace::new();
      let tsf = ts.create_tsf();
      let mut file = tsf.open_file();
      let expected = "a string to write";
      file.write_string("a string to write").unwrap();
      file.seek(SeekFrom::Start(0)).unwrap();
      let result = file.read_string().unwrap();
      assert_eq!(expected, result);
    }

    #[test]
    fn test_read_write_optional_path() {
        let ts = TestSpace::new();
        let base_path = ts.get_path();
        let ts2 = ts.create_child();
        let tsf = ts.create_tsf();
        let path_to_write = ts2.get_path();
        let optional_path_to_write = Some(path_to_write);
        // Write the optional path
        {
            let file = tsf.open_file();
            let mut writer = io::BufWriter::new(file);
            writer.write_optional_path(optional_path_to_write, base_path)
              .unwrap_or_else(|error| {
                  panic!("Writing optional path failed, error was {}", error);
              });
        }
        // Read the path back
        let file = tsf.open_file();
        let mut reader = io::BufReader::new(file);
        reader.seek(io::SeekFrom::Start(0)).unwrap();
        let read_result = reader.read_optional_path(base_path)
            .unwrap_or_else(|error| {
                panic!("read optional path failed, error was {}", error);
            });
        assert!(read_result.is_some());
        println!("Path that was written: {}", path_to_write.display());
        let read_result_path = read_result.unwrap();
        println!("Path that was read: {}", read_result_path.as_path().display());
        assert_eq!(path_to_write, read_result_path.as_path());

    }

    #[test]
    fn test_write_hash_at_position() {
        let ts = TestSpace::new();
        let mut tsf = ts.create_tsf();
        // Write 2k of random bytes to the test file
        tsf.write_random_bytes(2048);
        let test_data: [u8; 64] =
           [45, 64, 45, 64, 45, 64, 45, 64,
            45, 64, 45, 64, 45, 64, 45, 64,
            45, 64, 45, 64, 45, 64, 45, 64,
            45, 64, 45, 64, 45, 64, 45, 64,
            45, 64, 45, 64, 45, 64, 45, 64,
            45, 64, 45, 64, 45, 64, 45, 64,
            45, 64, 45, 64, 45, 64, 45, 64,
            45, 64, 45, 64, 45, 64, 45, 64];
        let hash = hash::Hash::from(test_data.to_vec());
        // Write test data to offset 10 in the test file
        {
            let mut file = tsf.open_file();
            file.write_hash_at_position(&hash, SeekFrom::Start(10)).unwrap();
        }
        let mut file = tsf.open_file();
        file.seek(SeekFrom::Start(10)).unwrap();
        let mut test_result = [0u8;64];
        file.read_exact(&mut test_result).unwrap();
        assert_eq!(test_data.as_ref(), test_result.as_ref());
    }

    #[test]
    fn test_write_u16_at_position() {
        use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

        let ts = TestSpace::new();
        let tsf = ts.create_tsf();
        let file = tsf.open_file();
        let mut writer = io::BufWriter::new(file);
        writer.write_u16::<LittleEndian>(12).unwrap();
        writer.write_u16::<LittleEndian>(12).unwrap();
        writer.write_u16::<LittleEndian>(12).unwrap();
        writer
            .write_u16_at_position(1000, SeekFrom::Start(0))
            .unwrap();

        let mut file = writer.into_inner().unwrap();
        file.seek(SeekFrom::Start(0)).unwrap();
        let value = file.read_u16::<LittleEndian>().unwrap();
        assert_eq!(value, 1000);
    }

    #[test]
    fn test_read_write_path() {
        use std::io::{BufReader, BufWriter};
        let ts = TestSpace::new();
        let ts2 = ts.create_child();
        let base_path = ts.get_path();
        let path_to_write = ts2.get_path();
        let tsf = ts.create_tsf();
        // Write test path
        {
            let test_file = tsf.open_file();
            let mut writer = BufWriter::new(test_file);
            writer
              .write_path(path_to_write, base_path)
              .unwrap_or_else(|err| {
                  panic!("Failed to write path, error was {}", err);
              });
        }
        // Read test path
        let test_file = tsf.open_file();
        let mut reader = BufReader::new(test_file);
        reader.seek(SeekFrom::Start(0)).unwrap_or_else(|err| {
            panic!("Failed to seek to start of test file, error was {}", err);
        });
        let read_result = reader.read_path(base_path).unwrap_or_else(|err| {
            panic!("Failed to read the test path, error was {}", err);
        });
        // Compare read result with what was written
        assert_eq!(read_result.as_path(), path_to_write);
    }

    #[test]
    fn test_read_write_hash() {
        use std::io::{BufReader, BufWriter};
        let known_hash: [u8; 64] = [
            0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11,
            0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF,
            0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11,
            0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF,
            0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF,
        ];
        let expected_hash = hash::Hash::from(known_hash.to_vec());
        let ts = TestSpace::new();
        let tsf = ts.create_tsf();
        {
            let file = tsf.open_file();
            let mut writer = BufWriter::new(file);
            writer.write_hash(&expected_hash)
              .unwrap_or_else(|err| {
                  panic!("Failed to write hash, error was {}", err);
              });
        }
        let file = tsf.open_file();
        let mut reader = BufReader::new(file);
        reader.seek(SeekFrom::Start(0)).unwrap_or_else(|err| {
            panic!("Failed to seek in temp test file, error was {}", err);
        });
        let result = reader.read_hash().unwrap_or_else(|err| {
            panic!("Failed to read the test hash, error was {}", err);
        });

        assert_eq!(result, expected_hash);
    }

    #[test]
    fn test_read_write_optional_hash() {
        use std::io::{BufReader, BufWriter};
        let known_hash: [u8; 64] = [
            0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11,
            0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF,
            0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11,
            0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF,
            0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF,
        ];
        let expected_hash = hash::Hash::from(known_hash.to_vec());
        let ts = TestSpace::new();
        let tsf = ts.create_tsf();
        {
            let file = tsf.open_file();
            let mut writer = BufWriter::new(file);
            writer.write_optional_hash(Some(&expected_hash))
              .unwrap_or_else(|err| {
                  panic!("Failed to write hash, error was {}", err);
              });
        }
        let file = tsf.open_file();
        let mut reader = BufReader::new(file);
        reader.seek(SeekFrom::Start(0)).unwrap_or_else(|err| {
            panic!("Failed to seek in temp test file, error was {}", err);
        });
        let result = reader.read_optional_hash().unwrap_or_else(|err| {
            panic!("Failed to read the test hash, error was {}", err);
        });
        assert!(Option::is_some(&result));
        assert_eq!(result.unwrap(), expected_hash);
    }

    #[test]
    fn test_read_write_optional_hash_none() {
        use std::io::{BufReader, BufWriter};
        let ts = TestSpace::new();
        let tsf = ts.create_tsf();
        {
            let file = tsf.open_file();
            let mut writer = BufWriter::new(file);
            writer.write_optional_hash(None)
              .unwrap_or_else(|err| {
                  panic!("Failed to write hash, error was {}", err);
              });
        }
        let file = tsf.open_file();
        let mut reader = BufReader::new(file);
        reader.seek(SeekFrom::Start(0)).unwrap_or_else(|err| {
            panic!("Failed to seek in temp test file, error was {}", err);
        });
        let result = reader.read_optional_hash().unwrap_or_else(|err| {
            panic!("Failed to read the test hash, error was {}", err);
        });
        assert!(Option::is_none(&result));
    }
}
