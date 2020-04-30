use crate::hash::Hash;
use super::SnapshotId;
// TODO: Custom impl of hash that just returns the Hash would work
// Using RC for both the path and the hash saves many allocations but creates multithreading issues
// TODO: Add location information to Hash variant


// impl Display for SnapshotId {
//   fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//     match self {
//       SnapshotId::Hash(hash) => write!(f, "Snapshot Id is a hash {}", hash),
//       SnapshotId::Indexed(index, hash) => write!(f, "Snapshot Id is an index {}", index),
//     }
    
//   }
// }

// When checking for equilivancy we only care about whether the hash is the same,
// It shouldn't matter if the snapshot that the ID represents is loaded or not
impl PartialEq for SnapshotId {
    /// Checks for equlivancy, only the hash is compared, whether the snapshot is loaded or not is disregarded
    fn eq(&self, other: &Self) -> bool {
        match self {
            SnapshotId::Located(hash, _) => {
                match other {
                    SnapshotId::Located(other_hash, _) => hash == other_hash,
                    SnapshotId::Indexed(_, other_hash) => hash == other_hash,
                    SnapshotId::NotLocated(other_hash) => hash == other_hash,
                }
            },
            SnapshotId::Indexed(_, hash) => {
                match other {
                    SnapshotId::Located(other_hash, _) => hash == other_hash,
                    SnapshotId::Indexed(_, other_hash) => hash == other_hash,
                    SnapshotId::NotLocated(other_hash) => hash == other_hash,
                }
            },
            SnapshotId::NotLocated(hash) => {
                match other {
                    SnapshotId::Located(other_hash, _) => hash == other_hash,
                    SnapshotId::Indexed(_, other_hash) => hash == other_hash,
                    SnapshotId::NotLocated(other_hash) => hash == other_hash,
                }
            }
        }
    }
}

impl SnapshotId {
    /// Is this ID currently indexed, meaning is the snapshot that this ID refers to already loaded
    pub fn is_indexed(&self) -> bool {
        match self {
            SnapshotId::Located(_, _) => false,
            SnapshotId::Indexed(_,_) => true,
            SnapshotId::NotLocated(_) => false,
        }
    }

    /// Returns a reference to the internal hash
    pub(crate) fn get_hash(&self) -> &Hash {
        match self {
            SnapshotId::Located(hash, _) => hash,
            SnapshotId::Indexed(_, hash) => hash,
            SnapshotId::NotLocated(hash) => hash,
        }
    }

    /// Removes the hash from the SnapshotId, the SnapshotId is consumed
    pub(crate) fn take_hash(self) -> Hash {
        match self {
            SnapshotId::Located(hash, _) => hash,
            SnapshotId::Indexed(_, hash) => hash,
            SnapshotId::NotLocated(hash) => hash,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::hash::Hash;
    use crate::snapshots::types::SnapshotId;
    use crate::snapshots::types::SnapshotLocation;

    #[test]
    fn snapshot_id_equal_test() {
        let test_hash = Hash::generate_random_hash();
        let id1 = SnapshotId::Indexed(4, test_hash.clone());
        let id2 = SnapshotId::Located(test_hash, SnapshotLocation::Simple);
        assert_eq!(id1, id2);
        let test_hash2 = Hash::generate_random_hash();
        let test_hash3 = Hash::generate_random_hash();
        let id3 = SnapshotId::Indexed(4, test_hash2);
        let id4 = SnapshotId::Indexed(4, test_hash3);
        assert_ne!(id3, id4);
    }
}