use std::string::String;
use crate::hash::Hash;
use super::{Snapshot, FileMetadata};
use blake2::{self, Digest};
use std::path;
use super::SnapshotBuilder;


// TODO: SnapshotBuilder should return self
impl SnapshotBuilder {
    pub fn change(snapshot: Snapshot) -> SnapshotBuilder {
        // TODO: When a snapshot is changed a new hash must be calculated and that hash distributed to its children and parents
        // TODO: But this is only a partial fix ?
        SnapshotBuilder::from(snapshot)
    }

    pub fn new() -> SnapshotBuilder {
        SnapshotBuilder {
            message: None,
            id: None,
            files: Vec::new(),
            children: Vec::new(),
            parent: None,
        }
    }

    pub fn set_message(&mut self, message: String) {
        self.message = Some(message);
    }

    pub fn add_file(&mut self, file_to_snapshot: FileMetadata) {
        self.files.push(file_to_snapshot);
    }

    pub fn add_files(&mut self, mut files_to_add: Vec<FileMetadata>) {
        self.files.append(&mut files_to_add);
    }

    pub fn remove_file(&mut self, file_to_remove: &path::Path) {
        if let Some(index) = self.files.iter().position(|metadata| metadata.path() == file_to_remove)
        {
            self.files.swap_remove(index);
        } else {
            // TODO: This needs to be handled better
            panic!("Attempted to remove a file from a snapshot that is not part of the snapshot");
        }
    }

    pub fn change_parent(&mut self, new_parent: Option<Hash>) {
        self.parent = new_parent;
    }

    pub fn add_child(&mut self, new_child: Hash) {
        self.children.push(new_child);
    }

    pub fn remove_child(&mut self, child_to_remove: &Hash) {
        if let Some(index) = self.children.iter().position(|hash| hash == child_to_remove)
        {
            self.children.swap_remove(index);
        } else {
            // TODO: This needs to be handled better
            panic!("Attempted to remove root hash that does not exist");
        }
    }

    pub fn build(self) -> Snapshot {
        if self.validate_snapshot() == false {
            panic!("The snapshot being built was not valid: {:?}", self);
        }
        // TODO: Only build a new ID if it doesn't already have one
        // TODO: Does a snapshot have to include a message for the hash
        // Deconstruct self
        let SnapshotBuilder { mut files, children, parent, message, id } = self;
        
        let message = message.unwrap();
        // TODO: If a snapshot is actually changed then a history of the changes needs to be kept, in addition how does the hash that identifies the snapshot change?
        if id.is_none() {
            // Building a new snapshot
            let new_id = SnapshotBuilder::generate_snapshot_id(message.as_bytes(), &mut files);
            return Snapshot::new(new_id, message, files, children, parent);
        } else {
            // Editing a snapshot
            // TODO: This wont work since it could lead to duplicate ID's
            return Snapshot::new(id.unwrap(), message, files, children, parent);
        }
    }

    /// Generates a snapshot hash to uniquely identify the snapshot
    fn generate_snapshot_id(message_bytes: &[u8], files: &mut Vec<FileMetadata>) -> Hash {
        // Sort by hash first
        // let files = self.files.clone();
        files.sort_by(|first, second| first.hash().cmp(&second.hash()));
        let mut hash = blake2::Blake2b::new();
        hash.input(message_bytes);
        for stored_file in files.iter() {
            hash.input(stored_file.hash().as_bytes());
        }
        let hash_result = hash.result();
        Hash::from(hash_result.to_vec())
    }

    /// Checks to see if the builder can create a snapshot based on the data it has
    fn validate_snapshot(&self) -> bool {
        if self.message.is_none() {
            // Can this be an empty string
            return false;
        }
        if self.files.len() == 0 {
            return false;
        }
        true
    }
}

impl From<Snapshot> for SnapshotBuilder {
    fn from(snapshot: Snapshot) -> Self {
        let (id, children, files, parent, message) = snapshot.breakup();
        SnapshotBuilder {
            message: Some(message),
            id: Some(id),
            parent,
            files,
            children,
        }
    }
}

impl std::fmt::Debug for SnapshotBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn string_from_optional_hash(hash_to_display: Option<&Hash>) -> String {
            return hash_to_display.map(|hash| String::from(hash)).unwrap_or(String::from("None"));
        }
        fn string_from_hash_vector(hash_vector: &[Hash]) -> String {
            let mut temp = String::new();
            for child in hash_vector {
                temp.push_str(String::from(format!("{},", child)).as_str());
            }
            temp
        }
        // let bug = self.files.into_iter().map(|x| String::from(x));
        writeln!(f, "Builder State").unwrap();
        
        writeln!(f, "Message is {}", self.message.as_ref().unwrap_or(&String::from("No Message"))).unwrap();
        writeln!(f, "Children are {}", string_from_hash_vector(self.children.as_slice())).unwrap();
        for child in self.children.iter().map(|hash| String::from(hash)) {
            write!(f, "{},", child).unwrap();
        }
        writeln!(f, "Parent is {}", string_from_optional_hash(self.parent.as_ref())).unwrap();
        for data in self.files.iter().map(|data| String::from(data)) {
            writeln!(f, "{}", data).unwrap();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{ SnapshotBuilder, FileMetadata, Snapshot };
    use testspace::TestSpace;
    use crate::hash::Hash;

    #[test]
    fn build_a_snapshot_test(){
        let mut builder = SnapshotBuilder::new();
        let mut ts = TestSpace::new();
        let mut file_list = ts.create_random_files(1, 2048);
        let file = file_list.remove(0);
        let test_hash = Hash::generate_random_hash();
        let test_parent = Hash::generate_random_hash();
        builder.set_message(String::from("A Message"));
        builder.add_file(FileMetadata::new(test_hash, 2048, file.clone(), 0));
        builder.change_parent(Some(test_parent.clone()));
        let result = builder.build();
        assert_eq!(result.get_message(), "A Message");
        assert_eq!(result.get_parent(), Some(test_parent).as_ref());
    }

    #[test]
    fn change_snapshot_test() {
        let test_snapshot = Snapshot::create_test_snapshot();
        let mut builder = SnapshotBuilder::change(test_snapshot);
        assert_ne!(builder.message, Some(String::from("test")));
        builder.set_message(String::from("test"));
        builder.change_parent(None);
        assert_eq!(builder.message, Some(String::from("test")));
        let snapshot = builder.build();
        assert_eq!(snapshot.get_parent(), None);
    }
}