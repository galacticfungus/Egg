use std::path;

mod file;
mod update;
mod location;
mod operation;
// mod recovery;

/// The atomic operation to perform, we store a path relative to the working directory
enum FileOperation {
    /// Files being replaced start in rw, then move to rc, but first files are moved from current to rp, finally rc is moved to current
    Replace(path::PathBuf),
    /// Files being created start in cw, then once complete are moved to cc
    Create(path::PathBuf),
    /// Files being stored are copied to sw, then moved to sc
    Store(path::PathBuf),
}

// TODO: AtomicUpdate needs to support versioning since a repository might be updated with an old interrupted operation
/// Responsible for making sure that all files are updated atomically
pub struct AtomicUpdate<'a> {
    atomic_jobs: Vec<FileOperation>,
    path_to_working: &'a path::Path,
    path_to_repository: &'a path::Path,
    path_to_create_working: path::PathBuf,
    path_to_create_complete: path::PathBuf,
    path_to_replace_working: path::PathBuf,
    path_to_replace_complete: path::PathBuf,
    path_to_replace_previous: path::PathBuf,
    path_to_replace_remove: path::PathBuf,
    path_to_store_working: path::PathBuf,
    path_to_store_complete: path::PathBuf,
}

pub(crate) enum AtomicLocation {
    Base,
    CreateWorking,
    CreateComplete,
    ReplaceWorking,
    ReplaceComplete,
    ReplacePrevious,
    ReplaceRemove,
    StoreWorking,
    StoreComplete,
}

#[cfg(test)]
mod tests {
    use testspace::TestSpace;
    use super::{AtomicUpdate, AtomicLocation};
    
    

    

    #[test]
    fn test_atomic_replace_init() {
        let ts = TestSpace::new();
        let ts2 = ts.create_child();
        let working_path = ts.get_path();
        let repository_path = ts2.get_path();
        AtomicUpdate::init(repository_path, working_path).expect("Failed to initialize atomic update");
        let temp_directory = repository_path.join(AtomicLocation::ReplaceWorking.get_path());
        let complete_directory = repository_path.join(AtomicLocation::ReplaceComplete.get_path());
        let previous_directory = repository_path.join(AtomicLocation::ReplacePrevious.get_path());
        let old_directory = repository_path.join(AtomicLocation::ReplaceRemove.get_path());
        assert!(temp_directory.exists());
        assert!(complete_directory.exists());
        assert!(previous_directory.exists());
        assert!(old_directory.exists());
    }

    #[test]
    fn test_atomic_create_init() {
        let ts = TestSpace::new();
        let ts2 = ts.create_child();
        let working_path = ts.get_path();
        let repository_path = ts2.get_path();
        AtomicUpdate::init(repository_path, working_path).expect("Failed to initialize atomic update");
        let complete_directory = repository_path.join(AtomicLocation::CreateComplete.get_path());
        let working_directory = repository_path.join(AtomicLocation::CreateWorking.get_path());
        assert!(working_directory.exists());
        assert!(complete_directory.exists());
    }

    #[test]
    fn test_atomic_store_init() {
        let ts = TestSpace::new();
        let ts2 = ts.create_child();
        let repository_path = ts2.get_path();
        let working_path = ts.get_path();
        AtomicUpdate::init(repository_path, working_path).expect("Failed to initialize atomic update");
        let complete_directory = repository_path.join(AtomicLocation::StoreComplete.get_path());
        let working_directory = repository_path.join(AtomicLocation::StoreWorking.get_path());
        assert!(working_directory.exists());
        assert!(complete_directory.exists());
    }
}