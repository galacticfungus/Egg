use blake2::{self, Blake2b, Digest};
use std::fs;
use std::path;
use std::vec::Vec;
use std::cmp::PartialEq;
use std::str;
use std::string::String;
use std::io::{self, Read};
use std::rc::Rc;
use crate::error::{Error, UnderlyingError};
use crate::snapshots::FileMetadata;

type Result<T> = std::result::Result<T, Error>;
impl Hash {
    pub fn hash_file(path_to_file: &path::Path) -> Result<Hash> {
        let file = match fs::File::open(path_to_file) {
            Ok(file) => file,
            Err(error) => {
            return Err(Error::file_error(Some(UnderlyingError::from(error)))
                .add_debug_message(format!("Failed to open a file when trying to hash file, path was {}", path_to_file.display()))
                .add_user_message(format!("Failed to open a file for reading, path was {}", path_to_file.display())));
            }
        };
        // TODO: Need more efficient buffering technique, in addition we can hash file while determining file type etc.
        let mut file_reader = io::BufReader::new(file);
        let mut buffer = Vec::with_capacity(2048); // TODO: This should probably be based on file size despite the extra system call
        let mut blake_hash = Blake2b::new();
        if let Err(error) = file_reader.read_to_end(&mut buffer) {
            return Err(Error::parsing_error(Some(UnderlyingError::from(error)))
                .add_debug_message(format!("hash_file failed to open the file being hashed, the path was {}", path_to_file.display()))
                .add_user_message(format!("Failed to open a file that needed to be hashed, the path was {}", path_to_file.display())));
        }
        blake_hash.input(&buffer);
        let hash_result = blake_hash.result();
        Ok(Hash::new(hash_result.as_slice()))
    }

    /// Generates a snapshot hash to uniquely identify the snapshot
    pub(crate) fn generate_snapshot_id(message: &str, files_in_snapshot: &mut [FileMetadata]) -> Hash {
        // Sort by hash first
        files_in_snapshot.sort_by(|first, second| first.hash().cmp(&second.hash()));
        let mut hash = blake2::Blake2b::new();
        hash.input(message.as_bytes());
        for stored_file in files_in_snapshot.iter() {
            hash.input(stored_file.hash().as_bytes());
        }
        let hash_result = hash.result();
        Hash::from(hash_result.to_vec())
    }
}
// http://fabiensanglard.net/git_code_review/diff.php
// Diff algorithms used in Git




#[derive(Clone)]
pub struct Hash {
    bytes: Rc<[u8;64]>,
}

// Need to implement Hash as the array too large
impl std::hash::Hash for Hash {
  // TODO: We already have a suitable hash so we need a pass through hasher?
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // Use the blake2 hash as our hash for the HashMap
        state.write(&self.bytes[.. self.bytes.len()]);
    }
}

impl std::fmt::Display for Hash {
    // 6F72222C33B9BF85E6379189116FD60D94B07E226FCEEF434E3376D6DD845759E36111483D990DD84AFCBF67F32B6871D825E65443A7CF61D043FE1D814C02ED
        // 6F72222C33B9BF85E6379189116FD6D94B07E226FCEEF434E3376D6DD845759E36111483D99DD84AFCBF67F32B6871D825E65443A7CF61D043FE1D814C2ED
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in self.bytes.iter() {
            write!(f, "{:02X}", byte).unwrap();
        }
        Ok(())
    }
}

// Need to implement Debug as the array too large
impl std::fmt::Debug for Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        //self.data[..].fmt(formatter)
        self.bytes[..].fmt(f)
    }
}

impl Hash {
    pub fn new(source: &[u8]) -> Hash {
        // TODO: Better array initialization - don't initialize twice
        // The copy here is probably not elided
        let mut bytes = Rc::new([0u8; 64]);
        let data = Rc::get_mut(&mut bytes).expect("Creating a new hash from a byte slice failed as the Rc was already shared");
        data.clone_from_slice(source);
        Hash {
            bytes,
        }
    }

    ///Takes a u8 and returns the hex representation for it, only the low 4 bits of the u8 are used
    fn map_to_char(value: u8) -> char {
        match value {
            0..=9 => {
                //48-57 for numbers
                char::from(value + 48)
            },
            10..=15 => {
                //65-70 for letters
                char::from(value + 55)
            },
            _ => {
                unreachable!("While converting a u8 to a hex representation, a value larger than 15 was encountered which should be impossible since each byte is split into 2 sections (nibble)")
            }
        }
    }

    // Helper method that converts a ASCII hex digit into its actually byte value ie F = 15
    fn hexdigit_to_byte(hex_digit: u8) -> u8 {
        match hex_digit {
        48..=57 => hex_digit - 48 as u8,
        65..=70 => hex_digit - 55 as u8,
        value => unreachable!("Any HashString should only contain letters or numbers found in a base 16 number, the value was {}", value)
        }
    }

    /// Get a slice to the bytes that make up the hash
    pub fn as_bytes(&self) -> &[u8] {
        // Derefence the box, then get a slice to the fixed array and return a reference to it
        &(*self.bytes)[..]
    }

    /// Return the number of bytes in the hash
    pub fn len(&self) -> usize {
        self.bytes.len()
    }
}

// TODO: All the byte operations done here can either be vectorized or cast as u64 before casting back

impl Ord for Hash {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.bytes.cmp(&(*other.bytes)[..])
    }
}

impl PartialOrd for Hash {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.bytes.partial_cmp(&(*other.bytes)[..])
    }
}

// Essentially uses partial Eq for equivalence
impl Eq for Hash {}

impl PartialEq for Hash {
    fn eq(&self, other: &Hash) -> bool {
        for index in 0..64 {
            if self.bytes[index] !=  other.bytes[index] {
                return false;
            }
        }
        true
    }
}

impl From<&[u8]> for Hash {
    fn from(byte_slice: &[u8]) -> Self {
        // TODO: If this is bottleneck then use unitialized memory in unsafe
        let mut bytes: [u8;64] = [0;64];
        bytes.copy_from_slice(byte_slice);
        Hash {
            bytes: Rc::new(bytes),
        }
    }
}

impl From<Vec<u8>> for Hash {
    fn from(buffer: Vec<u8>) -> Self {
        let mut bytes: [u8;64] = [0;64];
        bytes.copy_from_slice(buffer.as_slice());
        Hash {
        bytes: Rc::new(bytes),
        }
    }
}

impl From<&str> for Hash {
    fn from(raw_str: &str) -> Hash {
        let mut bytes = Rc::new([0u8; 64]);
        debug_assert!(raw_str.len() == 128, "Invalid string length, 128 characters (2 characters per byte) are required to convert a string into a 64 byte hash, length was: {}", raw_str.len());
        let string_as_bytes = raw_str.to_ascii_uppercase();
        let string_as_bytes = string_as_bytes.as_bytes();
        // Process the first byte character separately if the byte length is uneven
        let mut byte_index = 0;
        let data = Rc::get_mut(&mut bytes).expect("Could not get mutable access to a hash even though it was just created");
        for byte_pair in string_as_bytes.chunks(2) {
            debug_assert!(byte_pair[0].is_ascii_hexdigit(), "Invalid Hex digit: {}", byte_pair[0]);
            debug_assert!(byte_pair[1].is_ascii_hexdigit(), "Invalid Hex digit: {}", byte_pair[1]);
            let first_byte = Hash::hexdigit_to_byte(byte_pair[0]);
            let second_byte = Hash::hexdigit_to_byte(byte_pair[1]);
            let final_byte = (first_byte << 4) | second_byte;
            // let what = Rc::get_mut(&mut bytes);
            data[byte_index] = final_byte;
            byte_index += 1;
        }
        Hash {
            bytes: Rc::from(bytes),
        }
    }
}

impl From<String> for Hash {
    fn from(raw_string: String) -> Hash {
        Hash::from(raw_string.as_str())
    }
}

impl From<&Hash> for String {
    fn from(hash: &Hash) -> Self {
        let mut hash_string = String::with_capacity(hash.bytes.len() * 2);
        // Get a slice to underlying bytes
        let hash_bytes = &(*hash.bytes);
        for index in 0..hash.bytes.len() {
            // Every byte is two hex digits
            let second = hash_bytes[index] & 0b0000_1111u8; //Grab the low 4 bits
            let first = hash_bytes[index] >> 4 & 0b0000_1111u8; //Grab the high 4 bits
            hash_string.push(Hash::map_to_char(first));
            hash_string.push(Hash::map_to_char(second));
        }
        debug_assert!(hash_string.len() == 128);
        hash_string
    }
    // 6F72222C33B9BF85E6379189116FD60D94B07E226FCEEF434E3376D6DD845759E36111483D990DD84AFCBF67F32B6871D825E65443A7CF61D043FE1D814C02ED
    // 6F72222C33B9BF85E6379189116FD6-D94B07E226FCEEF434E3376D6DD845759E36111483D99-DD84AFCBF67F32B6871D825E65443A7CF61D043FE1D814C-2ED
}

impl From<Hash> for String {
    fn from(hash: Hash) -> Self {
        String::from(&hash)
    }
}

#[cfg(test)]
impl Hash {
    // NOTE: Only available when testing
    pub fn generate_random_hash() -> Hash {
        use rand::Rng;
        let rng = rand::thread_rng();
        let random_bytes: Vec<u8> = rng.sample_iter(rand::distributions::Standard).take(64).collect();
        Hash::from(random_bytes)
    }
  }

#[cfg(test)]
mod tests {
    use crate::hash::{Hash};
    use std::cmp::Ordering;

    #[test]
    fn test_hash_map_value_to_char() {
        assert_eq!('0', Hash::map_to_char(0));
        assert_eq!('A', Hash::map_to_char(0xA));
    }

    #[test]
    fn test_create_hash_string() {
        //Take a known hash
        let known_hash: [u8; 64] = [
        0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF,
        0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF,
        0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF,
        0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF,
        0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF,
        0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF,
        0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF,
        0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF
        ];
        let string_hash = Hash::from(
        "AB110AFFAB110AFFAB110AFFAB110AFFAB110AFFAB110AFFAB110AFFAB110AFFAB110AFFAB110AFFAB110AFFAB110AFFAB110AFFAB110AFFAB110AFFAB110AFF"
        );
        let hash_string = Hash::from(known_hash.as_ref());
        assert_eq!(hash_string, string_hash);
    }

    #[test]
    fn test_hash_get_bytes() {
        let known_hash: [u8; 64] = [
            0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF,
            0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF,
            0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF,
            0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF,
            0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF,
            0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF,
            0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF,
            0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF
        ];
        let string_hash = Hash::from(
        "AB110AFFAB110AFFAB110AFFAB110AFFAB110AFFAB110AFFAB110AFFAB110AFFAB110AFFAB110AFFAB110AFFAB110AFFAB110AFFAB110AFFAB110AFFAB110AFF"
        );
        let bytes = Hash::from(known_hash.as_ref());
        assert_eq!(string_hash, bytes);
    }

    #[test]
    fn test_lowercase_and_zero_digit_hash_conversions() {
        let known_hash: [u8;64] = [
            0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0xaa, 0xaa,
            0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0xaa, 0xaa,
            0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0xaa, 0xaa,
            0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0xaa, 0xaa,
            0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0xaa, 0xaa,
            0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0xaa, 0xaa,
            0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0xaa, 0xaa,
            0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0xaa, 0xaa,
        ];
        let string_hash = Hash::from(
        "aabbccddeeffaaaaaabbccddeeffaaaaaabbccddeeffaaaaaabbccddeeffaaaaaabbccddeeffaaaaaabbccddeeffaaaaaabbccddeeffaaaaaabbccddeeffaaaa"
        );
        let result = string_hash.as_bytes();
        assert_eq!(result, known_hash.as_ref());
        let known_hash2: [u8;64] = [
            0xa,0xb,0xc,0xd,0xe,0xf,0xa,0xb,
            0xa,0xb,0xc,0xd,0xe,0xf,0xa,0xb,
            0xa,0xb,0xc,0xd,0xe,0xf,0xa,0xb,
            0xa,0xb,0xc,0xd,0xe,0xf,0xa,0xb,
            0xa,0xb,0xc,0xd,0xe,0xf,0xa,0xb,
            0xa,0xb,0xc,0xd,0xe,0xf,0xa,0xb,
            0xa,0xb,0xc,0xd,0xe,0xf,0xa,0xb,
            0xa,0xb,0xc,0xd,0xe,0xf,0xa,0xb,
        ];
        let string_hash2 = Hash::from(
        "0a0b0c0d0e0f0a0b0a0b0c0d0e0f0a0b0a0b0c0d0e0f0a0b0a0b0c0d0e0f0a0b0a0b0c0d0e0f0a0b0a0b0c0d0e0f0a0b0a0b0c0d0e0f0a0b0a0b0c0d0e0f0a0b"
        );
        let result2 = string_hash2.as_bytes();
        assert_eq!(result2, known_hash2.as_ref());
    }

    #[test]
    fn test_hash_sort_comparisons() {
        let bytes: [u8;64] = [
            0x1,0xb,0xc,0xd,0xe,0xf,0xa,0xb,
            0xa,0xb,0xc,0xd,0xe,0xf,0xa,0xb,
            0xa,0xb,0xc,0xd,0xe,0xf,0xa,0xb,
            0xa,0xb,0xc,0xd,0xe,0xf,0xa,0xb,
            0xa,0xb,0xc,0xd,0xe,0xf,0xa,0xb,
            0xa,0xb,0xc,0xd,0xe,0xf,0xa,0xb,
            0xa,0xb,0xc,0xd,0xe,0xf,0xa,0xb,
            0xa,0xb,0xc,0xd,0xe,0xf,0xa,0xb,
        ];
        let bytes2: [u8;64] = [
            0xa,0xb,0xc,0xd,0xe,0xf,0xa,0xb,
            0xa,0xb,0xc,0xd,0xe,0xf,0xa,0xb,
            0xa,0xb,0xc,0xd,0xe,0xf,0xa,0xb,
            0xa,0xb,0xc,0xd,0xe,0xf,0xa,0xb,
            0xa,0xb,0xc,0xd,0xe,0xf,0xa,0xb,
            0xa,0xb,0xc,0xd,0xe,0xf,0xa,0xb,
            0xa,0xb,0xc,0xd,0xe,0xf,0xa,0xb,
            0xa,0xb,0xc,0xd,0xe,0xf,0xa,0xb,
        ];
        let hash = Hash::from(&bytes[..]);
        let hash2 = Hash::from(&bytes2[..]);
        // TODO: From std::convert::From<&[u8; 64]>
        let cmp_result = hash.cmp(&hash2);
        assert_eq!(cmp_result, Ordering::Less);
    }

    #[test]
    fn test_hash_equilivancy() {
        let bytes = [0x6F,0x72,0x22,0x2C,0x33,0xB9,0xBF,0x85,0xE6,0x37,0x91,0x89,0x11,0x6F,0xD6,0x0D,0x94,
        0xB0,0x7E,0x22,0x6F,0xCE,0xEF,0x43,0x4E,0x33,0x76,0xD6,0xDD,0x84,0x57,0x59,0xE3,0x61,0x11,0x48,0x3D,0x99,0x0D,0xD8,0x4A,0xFC,0xBF,0x67,0xF3,
        0x2B,0x68,0x71,0xD8,0x25,0xE6,0x54,0x43,0xA7,0xCF,0x61,0xD0,0x43,0xFE,0x1D,0x81,0x4C,0x02,0xED];
        let hash = Hash::from(&bytes[..]);
        let string = "6F72222C33B9BF85E6379189116FD60D94B07E226FCEEF434E3376D6DD845759E36111483D990DD84AFCBF67F32B6871D825E65443A7CF61D043FE1D814C02ED";
        let hash2 = Hash::from(string);
        assert_eq!(hash, hash2);
        let display_string = hash2.to_string();
        let hash_string = String::from(hash2);
        assert_eq!(string, hash_string);
        assert_eq!(display_string, hash_string);
    }
}
