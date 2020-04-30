use super::{AtomicUpdate, AtomicLocation, FileOperation};
use crate::error::{Error, UnderlyingError};
use std::path;
use std::fs;

impl<'a> AtomicUpdate<'a> {

    /// Initialize atomic file storage
    pub fn new(path_to_repository: &'a path::Path, path_to_working: &'a path::Path) -> Result<AtomicUpdate<'a>, Error> {
        // TODO: create_if_needed should return the path it created?
        fn create_if_needed(path_to_repository: &path::Path, atomic_path: AtomicLocation) -> Result<(), Error> {
            let test_directory = path_to_repository.join(atomic_path.get_path());
            if test_directory.exists() == false {
                if let Err(error) = fs::create_dir(test_directory.as_path()) {
                    return Err(Error::file_error(Some(UnderlyingError::from(error)))
                        .add_debug_message(format!("Failed to create a directory when initializing the Atomic Updater, path was {}", test_directory.display()))
                        .add_user_message(format!("Failed to create a directory when initializing the repository, the path was {}", test_directory.display())));
                }
            } else {
                // This case is problematic because it means that atomic has data from an incomplete operation but did not detect it as such
            }
            Ok(())
        }
        create_if_needed(path_to_repository, AtomicLocation::Base)?;
        let path_to_atomic = path_to_repository.join(AtomicLocation::Base.get_path());
        create_if_needed(path_to_atomic.as_path(), AtomicLocation::ReplaceWorking)?;
        create_if_needed(path_to_atomic.as_path(), AtomicLocation::ReplaceComplete)?;
        create_if_needed(path_to_atomic.as_path(), AtomicLocation::ReplacePrevious)?;
        create_if_needed(path_to_atomic.as_path(), AtomicLocation::ReplaceRemove)?;
        create_if_needed(path_to_atomic.as_path(), AtomicLocation::CreateWorking)?;
        create_if_needed(path_to_atomic.as_path(), AtomicLocation::CreateComplete)?;
        create_if_needed(path_to_atomic.as_path(), AtomicLocation::StoreWorking)?;
        create_if_needed(path_to_atomic.as_path(), AtomicLocation::StoreComplete)?;
        
        let au = AtomicUpdate {
            path_to_create_complete: path_to_atomic.join(AtomicLocation::CreateComplete.get_path()),
            path_to_create_working: path_to_atomic.join(AtomicLocation::CreateWorking.get_path()),
            path_to_replace_working: path_to_atomic.join(AtomicLocation::ReplaceWorking.get_path()),
            path_to_replace_complete: path_to_atomic.join(AtomicLocation::ReplaceComplete.get_path()),
            path_to_replace_previous: path_to_atomic.join(AtomicLocation::ReplacePrevious.get_path()),
            path_to_replace_remove: path_to_atomic.join(AtomicLocation::ReplaceRemove.get_path()),
            path_to_store_working: path_to_atomic.join(AtomicLocation::StoreWorking.get_path()),
            path_to_store_complete: path_to_atomic.join(AtomicLocation::StoreComplete.get_path()),
            path_to_repository,
            path_to_working,
            atomic_jobs: Vec::new(),
        };
        Ok(au)
    }

    pub fn load(path_to_working: &'a path::Path, path_to_repository: &'a path::Path) -> AtomicUpdate<'a> {
        let path_to_atomic = path_to_repository.join(AtomicLocation::Base.get_path());
        AtomicUpdate {
            path_to_create_complete: path_to_atomic.join(AtomicLocation::CreateComplete.get_path()),
            path_to_create_working: path_to_atomic.join(AtomicLocation::CreateWorking.get_path()),
            path_to_replace_working: path_to_atomic.join(AtomicLocation::ReplaceWorking.get_path()),
            path_to_replace_complete: path_to_atomic.join(AtomicLocation::ReplaceComplete.get_path()),
            path_to_replace_previous: path_to_atomic.join(AtomicLocation::ReplacePrevious.get_path()),
            path_to_replace_remove: path_to_atomic.join(AtomicLocation::ReplaceRemove.get_path()),
            path_to_store_working: path_to_atomic.join(AtomicLocation::StoreWorking.get_path()),
            path_to_store_complete: path_to_atomic.join(AtomicLocation::StoreComplete.get_path()),
            path_to_repository,
            path_to_working,
            atomic_jobs: Vec::new(),
        }
    }

    // Queue a replace file, returns a path that the new file should be written to
    pub fn queue_replace<S: Into<path::PathBuf>>(&mut self, file_to_replace: S) -> Result<path::PathBuf, Error> {

        // TODO: All queue operations must be done on absolute paths
        // TODO: All paths should be relative to the working path
        let file_to_replace = file_to_replace.into();
        if file_to_replace.is_absolute() == false {
            return Err(Error::invalid_parameter(None)
                .add_generic_message(format!("A path to a file to be replaced was not absolute, all paths to be processed by atomic must be absolute, the path was {}", file_to_replace.as_path().display())
            ));
        }
        let file_name = file_to_replace.file_name()
            .ok_or(Error::invalid_parameter(None).add_generic_message("Path did not have a file name"))?;
        let utf8_file_name = file_name.to_str()
            .ok_or(Error::invalid_parameter(None).add_generic_message("Path was not a valid UTF8 string"))?;
        let path_to_return = self.path_to_replace_working.join(utf8_file_name);
        self.atomic_jobs.push(FileOperation::Replace(file_to_replace));
        Ok(path_to_return)
    }

    // TODO: All these functions should take a path
    // Queue an atomic file create, returns a path that should be used as the path to the new file
    pub fn queue_create<S: Into<path::PathBuf>>(&mut self, file_to_create: S) -> Result<path::PathBuf, Error> {
        // TODO: All queue operations must be done on absolute paths
        let file_to_create = file_to_create.into();
        if file_to_create.is_absolute() == false {
            return Err(Error::invalid_parameter(None)
                .add_generic_message(format!("A path to a file to be created was not absolute, all paths to be processed by atomic must be absolute, the path was {}", file_to_create.as_path().display())
            ));
        }
        let file_name = file_to_create.file_name()
            .ok_or(Error::invalid_parameter(None).add_generic_message("Path did not have a file name"))?;
        let utf8_file_name = file_name.to_str()
            .ok_or(Error::invalid_parameter(None).add_generic_message("Path was not a valid UTF8 string"))?;
        let path_to_return = self.path_to_create_working.join(utf8_file_name);
        self.atomic_jobs.push(FileOperation::Create(file_to_create));
        Ok(path_to_return)
    }

    // Queue an atomic file store, returns a PathBuf that is where the file should be stored so that atomicUpdater can find it and place it into the repository atomically
    pub fn queue_store<S: Into<path::PathBuf>>(&mut self, file_to_store: S) -> Result<path::PathBuf, Error> {
        // TODO: All queue operations must be done on absolute paths
        // FIXME: This is complicated since later a file that is being stored could consist of many different files
        let file_to_store = file_to_store.into();
        let file_name = file_to_store.file_name()
            .ok_or(Error::invalid_parameter(None).add_generic_message("Path did not have a file name"))?;
        let utf8_file_name = file_name.to_str()
            .ok_or(Error::invalid_parameter(None).add_generic_message("Path was not a valid UTF8 string"))?;
        let path_to_return = self.path_to_store_working.join(utf8_file_name);
        self.atomic_jobs.push(FileOperation::Store(file_to_store));
        Ok(path_to_return)
    }

    /// Consumes the AtomicUpdate and updates each file pair that was
    /// registered atomically, call this when all file IO has been
    /// completed on the temporary files.
    pub fn complete(self) -> Result<(), Error> {
        // We process each stage one at a time progressing all files through them
        if let Err(error) = self.process_first_stage() {
            return Err(error.add_debug_message(format!("Stage one of the atomic update process failed, the repository is unchanged, the operation might be recoverable"))
                        .add_user_message(format!("Atomic update process failed, the repository is unchanged but the operation failed")));
        }
        // Stage 2 - move all snapshot files to previous
        self.process_second_stage()?;
        // Stage 3 - move all complete files to snapshot
        self.process_third_stage()?;
        // Stage 4 - move all previous files to bad
        self.process_fourth_stage()?;
        // Stage 5 - delete all bad files
        self.process_fifth_stage()?;
        Ok(())
    }

    fn process_first_stage(&self) -> Result<(), Error> {
        for job in self.atomic_jobs.as_slice() {
            let file_name = job.get_filename().map_err(|err| err.add_generic_message("During a stage one atomic operation"))?;
            match job {
                FileOperation::Create(_) => {
                    // Move from working to complete
                    let source_file = self.path_to_create_working.join(file_name);
                    let destination_file = self.path_to_create_complete.join(file_name);
                    fs::rename(source_file.as_path(), destination_file.as_path())
                        .map_err(|err| Error::file_error(Some(UnderlyingError::from(err)))
                            .add_debug_message(format!("A file rename failed while processing stage one of an atomic update, renaming {} to {} failed, the file operation was {:#?}", source_file.display(), destination_file.display(), job))
                            .add_user_message(format!("A file rename failed, failed to rename {} to {}", source_file.display(), destination_file.display()))
                    )?;
                },
                FileOperation::Replace(_) => {
                    // Move from working to complete
                    fs::rename(self.path_to_replace_working.join(file_name), self.path_to_replace_complete.join(file_name))
                        .map_err(|err| Error::file_error(Some(UnderlyingError::from(err)))
                            .add_generic_message("File rename failed during a stage one replace operation"))?;
                },
                FileOperation::Store(_) => {
                    fs::rename(self.path_to_store_working.join(file_name), self.path_to_store_complete.join(file_name))
                        .map_err(|err| Error::file_error(Some(UnderlyingError::from(err)))
                            .add_generic_message("File rename failed during a stage one store operation"))?;
                },
            }
        }
        Ok(())
    }
    // Move the file being replaced
    fn process_second_stage(&self) -> Result<(), Error> {
        for job in self.atomic_jobs.as_slice() {
            match job {
                FileOperation::Create(_) => {
                    // This is a no-op since we are not replacing a file
                },
                FileOperation::Replace(path_to_file) => {
                    // Move from current to previous
                    let file_name = job.get_filename()?;
                    let destination = self.path_to_replace_previous.join(file_name);
                    fs::rename(path_to_file, destination.as_path())
                        .map_err(|err| Error::file_error(Some(UnderlyingError::from(err)))
                            .add_user_message("An atomic operation failed (specifically a file rename) while trying to update the repository, the repository was not changed")
                            .add_debug_message(format!("A file rename failed while performing an atomic operation, the file {} was being renamed to {}",path_to_file.display(), destination.as_path().display())))?;
                },
                FileOperation::Store(_) => {
                    // No op since there is no file we are replacing
                },
            }
        }
        Ok(())
    }

    // The third stage will move files into the repository, which means after this point the operation is considered a success since only cleanup may be required
    fn process_third_stage(&self) -> Result<(), Error> {
        // TODO: Ensure that all paths are resolved to UNC paths
        for job in self.atomic_jobs.as_slice() {
            // FIXME: This doesn't include sub directories in the path that the file may have been restored from
            // let relative_path = job.get_relative_path(self.path_to_working)?;
            let file_name = job.get_filename()?;
            match job {
                FileOperation::Create(path_to_file) => {
                    // Move from complete to current
                    fs::rename(self.path_to_create_complete.join(file_name), path_to_file)
                        .map_err(|err| Error::file_error(Some(UnderlyingError::from(err)))
                            .add_generic_message("File rename failed during a stage three create operation"))?;
                },
                FileOperation::Replace(path_to_file) => {
                    // Move from complete to current
                    fs::rename(self.path_to_replace_complete.join(file_name), path_to_file)
                        .map_err(|err| Error::file_error(Some(UnderlyingError::from(err)))
                            .add_generic_message("File rename failed during a stage three replace operation"))?;
                },
                FileOperation::Store(_) => {
                    // FIXME: The original files path is not the same as the destination path in the case of storage
                    // Move from complete to storage
                    use crate::storage::LocalStorage;
                    let path_to_storage = self.path_to_repository.join(LocalStorage::DIRECTORY).join(file_name);
                    let path_to_complete_file = self.path_to_store_complete.join(file_name);
                    fs::rename(path_to_complete_file, path_to_storage.as_path())
                        .map_err(|err| Error::file_error(Some(UnderlyingError::from(err)))
                            .add_generic_message("File rename failed during a stage three store operation"))?;
                    
                },
            }
        }
        Ok(())
    }

    // The fourth stage moves files that have been replaced into a location to be removed 
    fn process_fourth_stage(&self) -> Result<(), Error> {
        // TODO: Ensure that all paths are resolved to UNC paths
        for job in self.atomic_jobs.as_slice() {
            match job {
                FileOperation::Create(_) => {
                    // Nothing to remove so no-op
                },
                FileOperation::Replace(_) => {
                    // Move from previous to remove
                    let file_name = job.get_filename()?;
                    fs::rename(self.path_to_replace_previous.join(file_name), self.path_to_replace_remove.join(file_name))
                        .map_err(|err| Error::file_error(Some(UnderlyingError::from(err)))
                            .add_generic_message("Rename failed during the fourth stage of an atomic operation"))?;
                },
                FileOperation::Store(_) => {
                    // Nothing to remove so no op
                },
            }
        }
        Ok(())
    }

    // The fifth stage removes files that were replaced
    fn process_fifth_stage(&self) -> Result<(), Error> {
        // TODO: Ensure that all paths are resolved to UNC paths
        for job in self.atomic_jobs.as_slice() {
            match job {
                FileOperation::Create(_) => {
                    // Nothing to remove so no-op
                },
                FileOperation::Replace(_) => {
                    let file_name = job.get_filename()?;
                    // Remove the file that was replaced
                    fs::remove_file(self.path_to_replace_remove.join(file_name))
                        .map_err(|err| Error::file_error(Some(UnderlyingError::from(err)))
                            .add_generic_message("Remove file failed during a stage five replace operation"))?;
                },
                FileOperation::Store(_) => {
                    // Nothing to remove so no op

                },
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use testspace::TestSpace;
    use super::{AtomicUpdate, AtomicLocation};


    #[test]
    fn test_atomic_init() {
        let ts = TestSpace::new().allow_cleanup(false);
        let ts2 = ts.create_child();
        let path_to_working = ts.get_path();
        let path_to_repository = ts2.get_path();
        let atomic = AtomicUpdate::new(path_to_repository, path_to_working).expect("Atomic init failed");
    }

    #[test]
    fn test_first_stage_queue_create() {
        // Queue some file creates and then write random data to them
        let mut ts = TestSpace::new();
        let ts2 = ts.create_child();
        let working_path = ts.get_path().to_path_buf();
        let repository_path = ts2.get_path().to_path_buf();
        let mut atomic = AtomicUpdate::new(repository_path.as_path(), working_path.as_path()).expect("Failed to initialize repository");
        for random_file in 0..5 {
            // Returns the path that we write data to
            let file_to_create = atomic.queue_create(format!("test{}",random_file)).expect("Failed to queue a create file");
            println!("Writing to path {:?}", file_to_create.as_path());
            ts.create_file(file_to_create, 2048);
        }
        // First stage will copy files that are being created from the cw directory to the cc directory
        atomic.process_first_stage().expect("First Stage failed");
        
        // Ensure the files exist
        for test_file in 0..5 {
            let path_to_test = repository_path.join(AtomicLocation::CreateComplete.get_path()).join(format!("test{}", test_file.to_string()));
            println!("Testing for file {}", path_to_test.display());
            assert!(path_to_test.exists());
        }
    }

    #[test]
    fn test_third_stage_queue_create() {
        // Queue some files to create
        let mut ts = TestSpace::new();
        let ts2 = ts.create_child();
        let working_path = ts.get_path().to_path_buf();
        let repository_path = ts2.get_path().to_path_buf();
        let mut atomic = AtomicUpdate::new(repository_path.as_path(), working_path.as_path()).expect("Failed to initialize repository");
        for random_file in 0..5 {
            // Returns the path that we write data to
            let file_to_create = atomic.queue_create(working_path.join(format!("test{}",random_file))).expect("Failed to queue a create file");
            ts.create_file(file_to_create, 2048);
        }
        // Move from working to complete
        atomic.process_first_stage().expect("First Stage failed");
        // Does nothing with files being created
        atomic.process_second_stage().expect("Second Stage failed");
        // Move from complete to current
        atomic.process_third_stage().expect("Second Stage failed");
        for test_file in 0..5 {
            let path_to_test = working_path.join(format!("test{}", test_file));
            println!("Testing for file {}", path_to_test.display());
            assert!(path_to_test.exists());
        }
    }

    #[test]
    fn test_first_stage_queue_replace() {
        let mut ts = TestSpace::new().allow_cleanup(false);
        let ts2 = ts.create_child();
        let working_path = ts.get_path().to_path_buf();
        let repository_path = ts2.get_path().to_path_buf();
        let mut atomic = AtomicUpdate::new(repository_path.as_path(), working_path.as_path()).expect("Failed to initialize repository");
        
        // Create the files that will be replaced
        for random_file in 0..5 {
            ts.create_file(working_path.join(format!("test{}", random_file)), 4096);
        }
        // Create the files that will replace the files in repository
        for random_file in 0..5 {
            // Returns the path that we write data to
            let file_to_replace = atomic.queue_replace(working_path.join(format!("test{}", random_file))).expect("Failed to queue a replace file");
            println!("File that is replacing {:?}", file_to_replace);
            ts.create_file(file_to_replace, 2048);
        }
        // First stage only moves from working to complete
        atomic.process_first_stage().expect("First Stage failed");
        for test_file in 0..5 {
            let path_to_test = repository_path.join(AtomicLocation::ReplaceComplete.get_path()).join(format!("test{}", test_file.to_string()));
            println!("Testing for file {}", path_to_test.display());
            assert!(path_to_test.exists());
        }
    }

    #[test]
    fn test_second_stage_queue_replace() {
        let mut ts = TestSpace::new().allow_cleanup(false);
        let ts2 = ts.create_child();
        let working_path = ts.get_path().to_path_buf();
        let repository_path = ts2.get_path().to_path_buf();
        let mut atomic = AtomicUpdate::new(repository_path.as_path(), working_path.as_path()).expect("Failed to initialize repository");
        
        // Create the files that will be replaced
        for random_file in 0..5 {
            let test_path = working_path.join(format!("test{}", random_file));
            println!("Creating test file in {:?}", test_path.as_path());
            ts.create_file(test_path, 4096);
        }
        // Create the files that will replace the files in repository
        for random_file in 0..5 {
            // Returns the path that we write data to
            let file_to_replace = atomic.queue_replace(working_path.join(format!("test{}",random_file))).expect("Failed to queue a file replace");
            println!("Replacement at {:?}", file_to_replace);
            ts.create_file(file_to_replace, 2048);
        }
        // First stage moves from working to complete
        atomic.process_first_stage().expect("First Stage failed");
        // Second stage moves from current to previous
        atomic.process_second_stage().expect("Second Stage failed");
        // Here we expect the files that were in working to now be in rp and working to be empty
        for test_file in 0..5 {
            // Repository/atomic/rp/test0
            let path_to_test = repository_path.join(AtomicLocation::ReplacePrevious.get_path()).join(format!("test{}", test_file.to_string()));
            println!("Testing for file {}", path_to_test.display());
            assert!(path_to_test.exists());
        }
    }

    #[test]
    fn test_third_stage_queue_replace() {
        let mut ts = TestSpace::new().allow_cleanup(false);
        let ts2 = ts.create_child();
        let working_path = ts.get_path().to_path_buf();
        let repository_path = ts2.get_path().to_path_buf();
        let mut atomic = AtomicUpdate::new(repository_path.as_path(), working_path.as_path()).expect("Failed to initialize repository");
        
        // Create the files that will be replaced
        for random_file in 0..5 {
            ts.create_file(working_path.join(format!("test{}", random_file)), 4096);
        }
        // Create the files that will replace the files in repository
        for random_file in 0..5 {
            // Returns the path that we write data to
            let file_to_create = atomic.queue_replace(working_path.join(format!("test{}",random_file))).expect("Failed to queue a file replace");
            ts.create_file(file_to_create, 2048);
        }
        // First stage only moves from working to complete
        atomic.process_first_stage().expect("First Stage failed");
        // Second stage moves from current to previous
        atomic.process_second_stage().expect("Second Stage failed");
        // Repository should have no files
        for test_file in 0..5 {
            let path_to_test = repository_path.join(format!("test{}", test_file));
            println!("Testing for file {}", path_to_test.display());
            assert_eq!(path_to_test.exists(), false);
        }
        // Move from complete to current
        atomic.process_third_stage().expect("Third stage failed");
        // Files have been replaced in the working directory, old files are in rp
        for test_file in 0..5 {
            let path_to_test = working_path.join(format!("test{}", test_file));
            println!("Testing for file {}", path_to_test.display());
            assert!(path_to_test.exists());
        }
    }

    #[test]
    fn test_fourth_stage_queue_replace() {
        let mut ts = TestSpace::new();
        let ts2 = ts.create_child();
        let working_path = ts.get_path().to_path_buf();
        let repository_path = ts2.get_path().to_path_buf();
        let mut atomic = AtomicUpdate::new(repository_path.as_path(), working_path.as_path()).expect("Failed to initialize repository");
        
        // Create the files that will be replaced
        for random_file in 0..5 {
            ts.create_file(working_path.join(format!("test{}", random_file)), 4096);
        }
        // Create the files that will replace the files in repository
        for random_file in 0..5 {
            // Returns the path that we write data to
            let file_to_create = atomic.queue_replace(working_path.join(format!("test{}",random_file))).expect("Failed to queue a file replace");
            ts.create_file(file_to_create, 2048);
        }
        // First stage only moves from working to complete
        atomic.process_first_stage().expect("First Stage failed");
        // TODO: Write a function that checks each stages success
        // Second stage moves from current to previous
        atomic.process_second_stage().expect("Second Stage failed");
        // Move from complete to current
        atomic.process_third_stage().expect("Third stage failed");
        // Move from previous to remove
        atomic.process_fourth_stage().expect("Fourth stage failed");
        // Check that files are waiting to be removed
        for test_file in 0..5 {
            let path_to_test = repository_path.join(format!("{}/test{}", AtomicLocation::ReplaceRemove.get_str(), test_file));
            println!("Testing for file {}", path_to_test.display());
            assert!(path_to_test.exists());
        }
    }

    #[test]
    fn test_fifth_stage_queue_replace() {
        let mut ts = TestSpace::new();
        let ts2 = ts.create_child();
        let working_path = ts.get_path().to_path_buf();
        let repository_path = ts2.get_path().to_path_buf();
        let mut atomic = AtomicUpdate::new(repository_path.as_path(), working_path.as_path()).expect("Failed to initialize repository");
        
        // Create the files that will be replaced
        for random_file in 0..5 {
            ts.create_file(working_path.join(format!("test{}", random_file)), 4096);
        }
        // Create the files that will replace the files in repository
        for random_file in 0..5 {
            // Returns the path that we write data to
            let file_to_create = atomic.queue_replace(working_path.join(format!("test{}",random_file))).expect("Failed to queue a file replace");
            ts.create_file(file_to_create, 2048);
        }
        // First stage only moves from working to complete
        atomic.process_first_stage().expect("First Stage failed");
        // Second stage moves from current to previous
        atomic.process_second_stage().expect("Second Stage failed");
        // Move from complete to current
        atomic.process_third_stage().expect("Third stage failed");
        // Move from previous to remove
        atomic.process_fourth_stage().expect("Fourth stage failed");
        // Remove the files
        atomic.process_fifth_stage().expect("Fourth stage failed");
        for test_file in 0..5 {
            let path_to_test = repository_path.join(format!("{}\\test{}", AtomicLocation::ReplaceRemove.get_str(), test_file));
            println!("Testing for file {}", path_to_test.display());
            assert_eq!(path_to_test.exists(), false);
        }
    }

    #[test]
    fn test_atomic_complete() {
        let mut ts = TestSpace::new();
        let ts2 = ts.create_child();
        let working_path = ts.get_path().to_path_buf();
        let repository_path = ts2.get_path().to_path_buf();
        let mut atomic = AtomicUpdate::new(repository_path.as_path(), working_path.as_path()).expect("Failed to initialize repository");
        // Create the files that will be replaced
        for random_file in 0..5 {
            ts.create_file(working_path.join(format!("test{}", random_file)), 4096);
        }
        // Create the files that will replace the files in repository
        for random_file in 0..5 {
            // Returns the path that we write data to
            let file_to_create = atomic.queue_replace(working_path.join(format!("test{}", random_file))).expect("Failed to queue a file replace");
            ts.create_file(file_to_create, 2048);
        }
        // Create the files that will be created in the repository
        for random_file in 0..5 {
            // Returns the path that we write data to
            let file_to_create = atomic.queue_create(working_path.join(format!("test_create{}", random_file))).expect("Failed to queue a file create");
            ts.create_file(file_to_create, 4096);
        }
        atomic.complete().expect("Atomic operation failed");
        // Check for created files
        for random_file in 0..5 {
            let file_to_create = working_path.join(format!("test_create{}", random_file));
            assert!(file_to_create.exists());
        }
        // Check for replaced files
        for random_file in 0..5 {
            let file_to_create = working_path.join(format!("test{}", random_file));
            assert!(file_to_create.exists());
        }
    }

    #[test]
    fn test_atomic_store() {
        use crate::storage::LocalStorage;
        let mut ts = TestSpace::new().allow_cleanup(false);
        let ts2 = ts.create_child();
        let working_path = ts.get_path().to_path_buf();
        let repository_path = ts2.get_path().to_path_buf();
        let mut atomic = AtomicUpdate::new(repository_path.as_path(), working_path.as_path()).expect("Failed to initialize repository");
        let fs = LocalStorage::initialize(repository_path.as_path()).expect("Failed to init file storage");
        // TODO: Redo this
        // Create the files that will be stored
        // for random_file in 0..5 {
        //     ts.create_file(working_path.join(format!("test{}", random_file)), 4096);
        //     let file_to_store = working_path.join(format!("test{}", random_file));
        //     // Queuing a store means that the files ends up in storage rather than in a relative path in the working directory
        //     let place_to_store = atomic.queue_store(working_path.join(format!("test{}", random_file))).expect("Failed to queue a file store");
        //     fs.store_file(file_to_store.as_path(), place_to_store.as_path()).expect("Failed to store file");
        // }
        // // Process stage one
        // atomic.process_first_stage().expect("Failed first stage");
        // // All files should have been moved to storage complete
        // for file in 0..5 {
        //     let path_to_check = atomic.path_to_store_complete.join(format!("test{}", file));
        //     assert!(path_to_check.exists());
        // }
        // atomic.process_third_stage().expect("All files should have been moved to storage");
        // // process stage three - Files should be in the storage folder in the repository
        // let storage_path = repository_path.join(LocalStorage::DIRECTORY);
        // for file in 0..5 {
        //     let path_to_check = storage_path.join(format!("test{}", file));
        //     assert!(path_to_check.exists());
        // }
    }

    // TODO: Advanced create
    // TODO: Advanced replace
    // TODO: Advanced Store
}