use std::path;
use std::fs;
use crate::error::{Error, UnderlyingError};
use std::fmt;

type Result<T> = std::result::Result<T, Error>;

/// The atomic operation to perform
enum FileOperation {
    /// Files ...
    Replace(String),
    /// Files being created start in cw, then once complete are moved to cc
    Create(String),
    /// Files being stored are copied to sw, then moved to sc
    Store(String),
}

enum AtomicRecoveryList<'a> {
    Create(Vec<&'a str>),
    Store(Vec<&'a str>),
    Replace(Vec<&'a str>),
}

impl<'a> AtomicRecoveryList<'a> {
    pub fn get_names(&'a self) -> &[&'a str] {
        match self {
            AtomicRecoveryList::Create(file_names) => file_names.as_slice(),
            AtomicRecoveryList::Store(file_names) => file_names.as_slice(),
            AtomicRecoveryList::Replace(file_names) => file_names.as_slice(),
        }
    }
}

enum AtomicRecoveryJob<'a> {
    StageOne(Vec<AtomicRecoveryList<'a>>),
    StageTwo(Vec<AtomicRecoveryList<'a>>),
    StageThree(Vec<AtomicRecoveryList<'a>>),
    StageFour(Vec<AtomicRecoveryList<'a>>),
    StageFive(Vec<AtomicRecoveryList<'a>>),
}

impl<'a> AtomicRecoveryJob<'a> {
    pub fn get_jobs(&'a self) -> &'a [AtomicRecoveryList] {
        match self {
            AtomicRecoveryJob::StageOne(jobs) => jobs,
            AtomicRecoveryJob::StageTwo(jobs) => jobs,
            AtomicRecoveryJob::StageThree(jobs) => jobs,
            AtomicRecoveryJob::StageFour(jobs) => jobs,
            AtomicRecoveryJob::StageFive(jobs) => jobs,
        }
    }

    pub fn get_create_jobs(&'a self) -> Option<&'a [&'a str]> {
        match self {
            AtomicRecoveryJob::StageOne(jobs) => {
                for job in jobs {
                    match job {
                        AtomicRecoveryList::Create(names) => return Some(names.as_slice()),
                        _ => {},
                    }
                }
                return None;
            },
            AtomicRecoveryJob::StageTwo(jobs) => {
                for job in jobs {
                    match job {
                        AtomicRecoveryList::Create(names) => return Some(names.as_slice()),
                        _ => {},
                    }
                }
                return None;
            },
            AtomicRecoveryJob::StageThree(jobs) => {
                for job in jobs {
                    match job {
                        AtomicRecoveryList::Create(names) => return Some(names.as_slice()),
                        _ => {},
                    }
                }
                return None;
            },
            AtomicRecoveryJob::StageFour(jobs) => {
                for job in jobs {
                    match job {
                        AtomicRecoveryList::Create(names) => return Some(names.as_slice()),
                        _ => {},
                    }
                }
                return None;
            },
            AtomicRecoveryJob::StageFive(jobs) => {
                for job in jobs {
                    match job {
                        AtomicRecoveryList::Create(names) => return Some(names.as_slice()),
                        _ => {},
                    }
                }
                return None;
            },
        }
    }

    pub fn get_store_jobs(&'a self) -> Option<&'a [&'a str]> {
        match self {
            AtomicRecoveryJob::StageOne(jobs) => {
                for job in jobs {
                    match job {
                        AtomicRecoveryList::Store(names) => return Some(names.as_slice()),
                        _ => {},
                    }
                }
                return None;
            },
            AtomicRecoveryJob::StageTwo(jobs) => {
                for job in jobs {
                    match job {
                        AtomicRecoveryList::Store(names) => return Some(names.as_slice()),
                        _ => {},
                    }
                }
                return None;
            },
            AtomicRecoveryJob::StageThree(jobs) => {
                for job in jobs {
                    match job {
                        AtomicRecoveryList::Store(names) => return Some(names.as_slice()),
                        _ => {},
                    }
                }
                return None;
            },
            AtomicRecoveryJob::StageFour(jobs) => {
                for job in jobs {
                    match job {
                        AtomicRecoveryList::Store(names) => return Some(names.as_slice()),
                        _ => {},
                    }
                }
                return None;
            },
            AtomicRecoveryJob::StageFive(jobs) => {
                for job in jobs {
                    match job {
                        AtomicRecoveryList::Store(names) => return Some(names.as_slice()),
                        _ => {},
                    }
                }
                return None;
            },
        }
    }

    pub fn get_replace_jobs(&'a self) -> Option<&'a [&'a str]> {
        match self {
            AtomicRecoveryJob::StageOne(jobs) => {
                for job in jobs {
                    match job {
                        AtomicRecoveryList::Replace(names) => return Some(names.as_slice()),
                        _ => {},
                    }
                }
                return None;
            },
            AtomicRecoveryJob::StageTwo(jobs) => {
                for job in jobs {
                    match job {
                        AtomicRecoveryList::Replace(names) => return Some(names.as_slice()),
                        _ => {},
                    }
                }
                return None;
            },
            AtomicRecoveryJob::StageThree(jobs) => {
                for job in jobs {
                    match job {
                        AtomicRecoveryList::Replace(names) => return Some(names.as_slice()),
                        _ => {},
                    }
                }
                return None;
            },
            AtomicRecoveryJob::StageFour(jobs) => {
                for job in jobs {
                    match job {
                        AtomicRecoveryList::Replace(names) => return Some(names.as_slice()),
                        _ => {},
                    }
                }
                return None;
            },
            AtomicRecoveryJob::StageFive(jobs) => {
                for job in jobs {
                    match job {
                        AtomicRecoveryList::Replace(names) => return Some(names.as_slice()),
                        _ => {},
                    }
                }
                return None;
            },
        }
    }
}

impl std::fmt::Display for FileOperation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FileOperation::Create(_) => write!(f, "FileOperation::Create"),
            FileOperation::Replace(_) => write!(f, "FileOperation::Replace"),
            FileOperation::Store(_) => write!(f, "FileOperation::Store"),
        }
    }
}

impl std::fmt::Debug for FileOperation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FileOperation::Create(file_name) => write!(f, "FileOperation::Create({})", file_name),
            FileOperation::Replace(file_name) => write!(f, "FileOperation::Replace({})", file_name),
            FileOperation::Store(file_name) => write!(f, "FileOperation::Store({})", file_name),
        }
    }
}
// TODO: AtomicUpdate needs to support versioning since a repository might be updated with an old interrupted operation
/// Responsible for making sure that all files are updated atomically
pub struct AtomicUpdate<'a> {
    // All paths are processed relative to the repository path
    atomic_jobs: Vec<FileOperation>,
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

pub(crate) enum AtomicPaths {
    CreateWorking,
    CreateComplete,
    ReplaceWorking,
    ReplaceComplete,
    ReplacePrevious,
    ReplaceRemove,
    StoreWorking,
    StoreComplete,
}

impl AtomicPaths {
    fn get_path(&self) -> &path::Path {
        path::Path::new(self.get_str())
    }

    fn get_str(&self) -> &str {
        match self {
            AtomicPaths::CreateComplete => "cc",
            AtomicPaths::CreateWorking => "cw",
            AtomicPaths::ReplaceWorking => "rw",
            AtomicPaths::ReplaceComplete => "rc",
            AtomicPaths::ReplacePrevious => "rp",
            AtomicPaths::ReplaceRemove => "rr",
            AtomicPaths::StoreWorking => "sw",
            AtomicPaths::StoreComplete => "sc",
        }
    }
}

impl<'a> AtomicUpdate<'a> {
    pub fn recover(path_to_repository: &path::Path) -> Result<()> {
        // Construct paths
        let cw = path_to_repository.join(AtomicPaths::CreateWorking.get_path());
        let cc = path_to_repository.join(AtomicPaths::CreateComplete.get_path());
        let sw = path_to_repository.join(AtomicPaths::StoreWorking.get_path());
        let sc = path_to_repository.join(AtomicPaths::StoreComplete.get_path());
        let rw = path_to_repository.join(AtomicPaths::ReplaceWorking.get_path());
        let rc = path_to_repository.join(AtomicPaths::ReplaceComplete.get_path());
        let rp = path_to_repository.join(AtomicPaths::ReplacePrevious.get_path());
        let rr = path_to_repository.join(AtomicPaths::ReplaceRemove.get_path());

        // TODO: If get_files is called elsewhere then some context may need to be added
        let cw_list = AtomicUpdate::get_files(cw.as_path())?;
        let cc_list = AtomicUpdate::get_files(cc.as_path())?;
        let sw_list = AtomicUpdate::get_files(sw.as_path())?;
        let sc_list = AtomicUpdate::get_files(sc.as_path())?;
        let rw_list = AtomicUpdate::get_files(rw.as_path())?;
        let rc_list = AtomicUpdate::get_files(rc.as_path())?;
        let rp_list = AtomicUpdate::get_files(rp.as_path())?;
        let rr_list = AtomicUpdate::get_files(rr.as_path())?;
        // First can we restore?
        let restore_possible = AtomicUpdate::can_restore(sw_list.len(), sc_list.len(), cw_list.len(), cc_list.len(), rw_list.len(), rp_list.len());
        if restore_possible {
            
            // let jobs_to_execute = AtomicUpdate::build_recovery_job();
            
            

            // Execute recovery jobs
        } else {
            // Remove all files in all atomic paths and report that operation did not complete
            // This is a dangerous operation and must be cleaned in the reverse order of stages to ensure we dont trick recoverer into 
            // thinking that an operation was more successful than it was
            // TODO: We should only need to clean the working directories since a single stage is completed one at a time
            AtomicUpdate::clean_directory(rp.as_path())?;
            AtomicUpdate::clean_directory(rr.as_path())?;
            AtomicUpdate::clean_directory(cc.as_path())?;
            AtomicUpdate::clean_directory(sc.as_path())?;
            AtomicUpdate::clean_directory(rc.as_path())?;
            AtomicUpdate::clean_directory(cw.as_path())?;
            AtomicUpdate::clean_directory(sw.as_path())?;
            AtomicUpdate::clean_directory(rw.as_path())?;
        }
        Ok(())
    }

    fn build_recovery_job(cw_list: Vec<String>, cc_list: Vec<String>, sc_list: Vec<String>, sw_list: Vec<String>, rp_list: Vec<String>, rw_list: Vec<String>, rc_list: Vec<String>, rr_list: Vec<String>) -> Vec<AtomicRecoveryJob<'a>> {
        // TODO: Lists can be partial, for instance 4 items in working, 3 items in previous means 7 items for complete
        // Files for Stage two create, store and replace
        let mut rp_ref_list: Vec<&str> = rp_list.iter().map(|s| s.as_str()).collect();
        let mut rw_ref_list: Vec<&str> = rw_list.iter().map(|s| s.as_str()).collect();
        let mut rc_ref_list: Vec<&str> = rc_list.iter().map(|s| s.as_str()).collect();
        let mut rr_ref_list: Vec<&str> = rr_list.iter().map(|s| s.as_str()).collect();
        let mut sw_ref_list: Vec<&str> = sw_list.iter().map(|s| s.as_str()).collect();
        let mut sc_ref_list: Vec<&str> = sc_list.iter().map(|s| s.as_str()).collect();
        let mut cw_ref_list: Vec<&str> = cw_list.iter().map(|s| s.as_str()).collect();
        let mut cc_ref_list: Vec<&str> = cc_list.iter().map(|s| s.as_str()).collect();
        
        
        // let mut stage_three_jobs = Vec::new();
        // let mut stage_four_jobs = Vec::new();
        // let mut stage_five_jobs = Vec::new();
        // Stage One still needs to be performed as it may have only partially been completed - ie move all working to completed
        let stage_one_jobs = AtomicUpdate::create_stage_one_list(cw_ref_list.clone(), sw_ref_list.clone(), rw_ref_list.clone());
        let stage_two_jobs = AtomicUpdate::create_stage_two_list(rp_ref_list.clone(), rc_ref_list.clone(), rw_ref_list.clone());
        let stage_three_jobs = AtomicUpdate::create_stage_three_list(rw_ref_list.clone(), rc_ref_list.clone(), sw_ref_list.clone(), sc_ref_list.clone(), cw_ref_list.clone(), cc_ref_list.clone());
        

        // Files for stage four create store and replace
        // Files for stage five create store and replace
        Vec::new()
    }

    fn create_stage_three_list(mut rw_list: Vec<&'a str>, mut rc_list: Vec<&'a str>, mut sw_list: Vec<&'a str>, mut sc_list: Vec<&'a str>, mut cw_list: Vec<&'a str>, mut cc_list: Vec<&'a str>) -> AtomicRecoveryJob<'a> {
        // Move all the files in completed to the repository
        sc_list.append(&mut sw_list);
        cc_list.append(&mut cw_list);
        rc_list.append(&mut rw_list);
        let mut stage_three_jobs = Vec::new();
        let stage_two_create = AtomicRecoveryList::Create(cc_list);
        let stage_two_store = AtomicRecoveryList::Store(sc_list);
        let stage_two_replace = AtomicRecoveryList::Replace(rc_list);
        stage_three_jobs.push(stage_two_create);
        stage_three_jobs.push(stage_two_store);
        stage_three_jobs.push(stage_two_replace);
        AtomicRecoveryJob::StageThree(stage_three_jobs)
    }

    /// Create file list for stage two
    fn create_stage_two_list(rp_list: Vec<&'a str>, mut rc_list: Vec<&'a str>, mut rw_list: Vec<&'a str>) -> AtomicRecoveryJob<'a> {
        /// Subtracts a vector from another vector modifying the resultant vector in place
        fn subtract_vector(vector: &mut Vec<&str>, vector_to_subtract: Vec<&str>) {
            let mut i = 0;
            while i != vector.len() {
                if vector_to_subtract.contains(&vector[i]) {
                    vector.remove(i);
                } else {
                    i += 1;
                }
            }
        }
        // Move files from repository to previous - The complete list for files can be found from (complete + working) - files already in previous, if working contains files then there should be no files in previous
        // TODO: Assuming that working is empty here may not be correct, since if some files are moved to complete but not all then recovery is still possible but working is not empty
        rc_list.append(&mut rw_list);
        println!("Appended List: {:?}", rc_list);
        subtract_vector(&mut rc_list, rp_list.clone());
        println!("Subtracted Result: {:?} - {:?} = {:?}", rc_list, rp_list, rc_list.clone());
        //rc_list.drain_filter(|file_name| rp_list.contains(file_name));
        // TODO: Working must be empty if previous has files
        let mut stage_two_jobs = Vec::new();
        let stage_two_replace = AtomicRecoveryList::Replace(rc_list);
        stage_two_jobs.push(stage_two_replace);
        AtomicRecoveryJob::StageTwo(stage_two_jobs)
    }

    /// Create file list for stage one
    fn create_stage_one_list(cw_list: Vec<&'a str>, sw_list: Vec<&'a str>, rw_list: Vec<&'a str>) -> AtomicRecoveryJob<'a> {
        let mut stage_one_jobs = Vec::new();
        let stage_one_create = AtomicRecoveryList::Create(cw_list);
        let stage_one_store = AtomicRecoveryList::Store(sw_list);
        let stage_one_replace = AtomicRecoveryList::Replace(rw_list);
        stage_one_jobs.push(stage_one_create);
        stage_one_jobs.push(stage_one_store);
        stage_one_jobs.push(stage_one_replace);
        AtomicRecoveryJob::StageOne(stage_one_jobs)
    }

    fn clean_directory(path_to_directory: &path::Path) -> Result<()> {
        let items_found = match fs::read_dir(path_to_directory) {
            Ok(result) => result,
            Err(error) => unimplemented!(),
        };
        for item in items_found {
            let valid_item = match item {
                Ok(valid_item) => valid_item,
                Err(error) => unimplemented!(),
            };
            match valid_item.file_type() {
                Ok(file_type) if file_type.is_file() => {
                    if let Err(error) = fs::remove_file(valid_item.path()) {
                        unimplemented!();
                    }
                },
                Ok(file_type) => {
                    // Unknown item type
                },
                Err(error) => unimplemented!(),
            }
        }
        Ok(())
    }

    /// Get a list of file names for the given directory
    fn get_files(path_to_directory: &path::Path) -> Result<Vec<String>> {
        let mut results = Vec::new();
        let items_found = match fs::read_dir(path_to_directory) {
            Ok(result) => result,
            Err(error) => unimplemented!(),
        };
        for item in items_found {
            let valid_item = match item {
                Ok(valid_item) => valid_item,
                Err(error) => unimplemented!(),
            };
            match valid_item.file_type() {
                Ok(file_type) if file_type.is_file() => {
                    // let file_name = String::from(valid_item.path().file_name().unwrap().to_str().unwrap());
                    let file_name = match valid_item.path().file_name() {
                        Some(os_name) => {
                            match os_name.to_str() {
                                Some(file_name) => String::from(file_name),
                                None => unimplemented!(), // Invalid UTF8
                            }
                        },
                        None => unimplemented!(), // No file name
                    };
                    results.push(file_name);
                },
                Ok(file_type) => {
                    // Unknown item type
                },
                Err(error) => unimplemented!(),
            }
        }
        Ok(results)
    }

    /// Checks to see if the operations that were under way are recoverable
    fn can_restore(sw_count: usize, sc_count: usize, cw_count: usize, cc_count: usize, rw_count: usize, rp_count: usize) -> bool {
        fn test_working_complete(working_count: usize, complete_count: usize, replace_previous: usize) -> bool {
            match (working_count > 0, complete_count > 0, replace_previous > 0) {
                (_, true, _) => true,      // If complete contains files at all then recovery is possible
                (_, _, true) => true,      // If previous contains files at all then recovery is possible
                (true, false, false) => false, // If only working contains files then recovery is not possible
                (false, false, false) => true, // No files found so either the operation was interrupted at a later stage or that operation wasn't performed
            }
        }
        let store_recoverable = test_working_complete(sw_count, sc_count, rp_count);
        let create_recoverable = test_working_complete(cw_count, cc_count, rp_count);
        // Explicit test to avoid confusion over naming
        // TODO: This is flawed since if stage 2 is under way, both store and create operations will return that they can't complete since working may have files but complete will be empty, stage 2 is only used in replace
        // Explicitly test rp - if rp > 0 then working is fine
        let replace_recoverable = match (rw_count > 0, rp_count > 0) {
            (_, true) => true,      // Any files found in previous folder means that stage 2 has started and all writes have finished
            (true, false) => false, // Only found files in the working folder so not recoverable
            (false, false) => true, // No files found so either the operation was interrupted at a later stage or that operation wasn't performed
        };
        // Are all operations recoverable
        store_recoverable && create_recoverable && replace_recoverable
    }

    pub fn was_interrupted(path_to_repository: &path::Path) -> bool {
        // Construct paths
        let cw = path_to_repository.join(AtomicPaths::CreateWorking.get_path());
        let cc = path_to_repository.join(AtomicPaths::CreateComplete.get_path());
        let sw = path_to_repository.join(AtomicPaths::StoreWorking.get_path());
        let sc = path_to_repository.join(AtomicPaths::StoreComplete.get_path());
        let rw = path_to_repository.join(AtomicPaths::ReplaceWorking.get_path());
        let rc = path_to_repository.join(AtomicPaths::ReplaceComplete.get_path());
        let rp = path_to_repository.join(AtomicPaths::ReplacePrevious.get_path());
        let rr = path_to_repository.join(AtomicPaths::ReplaceRemove.get_path());
        // Check for any files in the above paths
        AtomicUpdate::contains_files(cw.as_path()) || AtomicUpdate::contains_files(cc.as_path()) || AtomicUpdate::contains_files(sw.as_path()) || AtomicUpdate::contains_files(sc.as_path()) || 
            AtomicUpdate::contains_files(rw.as_path()) || AtomicUpdate::contains_files(rc.as_path()) || AtomicUpdate::contains_files(rp.as_path()) || AtomicUpdate::contains_files(rr.as_path())
    }

    fn contains_files(path_to_search: &path::Path) -> bool {
        let contents_of_path = match fs::read_dir(path_to_search) {
            Ok(contents_of_path) => contents_of_path,
            Err(error) => unimplemented!(),
        };
        if contents_of_path.count() > 0 {
            return true;
        }
        return false;
    }
}

impl<'a> AtomicUpdate<'a> {

    pub fn new(path_to_repository: &path::Path) -> AtomicUpdate {
        // TODO: Create all needed paths here

        // Create Paths are cw (create_working), cc (create_complete)
        
        // TODO: Need to unc these paths
        // AtomicPaths::CreateComplete.get_path()
        AtomicUpdate {
            path_to_create_complete: path_to_repository.join(AtomicPaths::CreateComplete.get_path()),
            path_to_create_working: path_to_repository.join(AtomicPaths::CreateWorking.get_path()),
            path_to_replace_working: path_to_repository.join(AtomicPaths::ReplaceWorking.get_path()),
            path_to_replace_complete: path_to_repository.join(AtomicPaths::ReplaceComplete.get_path()),
            path_to_replace_previous: path_to_repository.join(AtomicPaths::ReplacePrevious.get_path()),
            path_to_replace_remove: path_to_repository.join(AtomicPaths::ReplaceRemove.get_path()),
            path_to_store_working: path_to_repository.join(AtomicPaths::StoreWorking.get_path()),
            path_to_store_complete: path_to_repository.join(AtomicPaths::StoreComplete.get_path()),
            path_to_repository,
            atomic_jobs: Vec::new(),
        }
    }

    // Queue a replace file, returns a path that the new file should be written to
    pub fn queue_replace<S: Into<String>>(&mut self, file_name: S) -> path::PathBuf {
        let file_name = file_name.into();
        let path_to_return = self.path_to_replace_working.join(file_name.as_str());
        self.atomic_jobs.push(FileOperation::Replace(file_name));
        return path_to_return;
    }

    // Queue an atomic file create, returns a path that should be used as the path to the new file
    pub fn queue_create<S: Into<String>>(&mut self, file_name: S) -> path::PathBuf {
        let file_name = file_name.into();
        let path_to_return = self.path_to_create_working.join(file_name.as_str());
        self.atomic_jobs.push(FileOperation::Create(file_name));
        return path_to_return;
    }

    // Queue an atomic file store, returns a PathBuf that is where the file should be stored so that atomicUpdater can find it and place it into the repository atomically
    pub fn queue_store<S: Into<String>>(&mut self, file_to_store: S) -> path::PathBuf {
        // FIXME: This is complicated since later a file that is being stored could consist of many different files
        let file_to_store = file_to_store.into();
        let path_to_return = self.path_to_store_working.join(file_to_store.as_str());
        self.atomic_jobs.push(FileOperation::Store(file_to_store));
        return path_to_return;
    }
    /// Initialize atomic file storage
    pub fn init(path_to_repository: &path::Path) -> Result<AtomicUpdate> {
        // TODO: create_if_needed should return the path it created
        fn create_if_needed(path_to_repository: &path::Path, atomic_path: AtomicPaths) -> Result<()> {
            let test_directory = path_to_repository.join(atomic_path.get_path());
            if test_directory.exists() == false {
                if let Err(error) = fs::create_dir(test_directory.as_path()) {
                    return Err(Error::file_error(Some(UnderlyingError::from(error)))
                        .add_debug_message(format!("Failed to create a directory when initializing the Atomic Updater, path was {}", test_directory.display()))
                        .add_user_message(format!("Failed to create a directory when initializing the repository, the path was {}", test_directory.display())));
                }
            }
            Ok(())
        }
        create_if_needed(path_to_repository, AtomicPaths::ReplaceWorking)?;
        create_if_needed(path_to_repository, AtomicPaths::ReplaceComplete)?;
        create_if_needed(path_to_repository, AtomicPaths::ReplacePrevious)?;
        create_if_needed(path_to_repository, AtomicPaths::ReplaceRemove)?;
        create_if_needed(path_to_repository, AtomicPaths::CreateWorking)?;
        create_if_needed(path_to_repository, AtomicPaths::CreateComplete)?;
        create_if_needed(path_to_repository, AtomicPaths::StoreWorking)?;
        create_if_needed(path_to_repository, AtomicPaths::StoreComplete)?;
        
        let au = AtomicUpdate {
            path_to_create_complete: path_to_repository.join(AtomicPaths::CreateComplete.get_path()),
            path_to_create_working: path_to_repository.join(AtomicPaths::CreateWorking.get_path()),
            path_to_replace_working: path_to_repository.join(AtomicPaths::ReplaceWorking.get_path()),
            path_to_replace_complete: path_to_repository.join(AtomicPaths::ReplaceComplete.get_path()),
            path_to_replace_previous: path_to_repository.join(AtomicPaths::ReplacePrevious.get_path()),
            path_to_replace_remove: path_to_repository.join(AtomicPaths::ReplaceRemove.get_path()),
            path_to_store_working: path_to_repository.join(AtomicPaths::StoreWorking.get_path()),
            path_to_store_complete: path_to_repository.join(AtomicPaths::StoreComplete.get_path()),
            path_to_repository,
            atomic_jobs: Vec::new(),
        };
        Ok(au)
    }

    /// Consumes the AtomicUpdate and updates each file pair that was
    /// registered atomically, call this when all file IO has been
    /// completed on the temporary files.
    pub fn complete(self) -> Result<()> {
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

    fn process_first_stage(&self) -> Result<()> {
        for job in self.atomic_jobs.as_slice() {
            match job {
                FileOperation::Create(file_name) => {
                    // Move from working to complete
                    let source_file = self.path_to_create_working.join(file_name.as_str());
                    let destination_file = self.path_to_create_complete.join(file_name.as_str());
                    if let Err(error) = fs::rename(source_file.as_path(), destination_file.as_path()) {
                        return Err(Error::file_error(Some(UnderlyingError::from(error)))
                                        .add_debug_message(format!("A file rename failed while processing stage one of an atomic update, renaming {} to {} failed, the file operation was {:#?}", source_file.display(), destination_file.display(), job))
                                        .add_user_message(format!("A file rename failed, failed to rename {} to {}", source_file.display(), destination_file.display())));
                    }
                },
                FileOperation::Replace(file_name) => {
                    // Move from working to complete
                    if let Err(error) = fs::rename(self.path_to_replace_working.join(file_name.as_str()), self.path_to_replace_complete.join(file_name.as_str())) {
                        unimplemented!();
                    }
                },
                FileOperation::Store(file_name) => {
                    if let Err(error) = fs::rename(self.path_to_store_working.join(file_name.as_str()), self.path_to_store_complete.join(file_name.as_str())) {
                        unimplemented!();
                    }
                },
            }
        }
        Ok(())
    }
    // Move the file being replaced
    fn process_second_stage(&self) -> Result<()> {
        for job in self.atomic_jobs.as_slice() {
            match job {
                FileOperation::Create(_) => {
                    // This is a no-op since we are not replacing a file
                },
                FileOperation::Replace(file_name) => {
                    // Move from current to previous
                    if let Err(error) = fs::rename(self.path_to_repository.join(file_name.as_str()), self.path_to_replace_previous.join(file_name.as_str())) {
                        unimplemented!();
                    }
                },
                FileOperation::Store(_) => {
                    // No op since there is no file we are replacing
                },
            }
        }
        Ok(())
    }

    // The third stage will move files into the repository, which means after this point the operation is considered a success since only cleanup may be required
    fn process_third_stage(&self) -> Result<()> {
        // TODO: Ensure that all paths are resolved to UNC paths
        for job in self.atomic_jobs.as_slice() {
            match job {
                FileOperation::Create(file_name) => {
                    // Move from complete to current
                    if let Err(error) = fs::rename(self.path_to_create_complete.join(file_name.as_str()), self.path_to_repository.join(file_name.as_str())) {
                        unimplemented!();
                    }
                },
                FileOperation::Replace(file_name) => {
                    // Move from complete to current
                    if let Err(error) = fs::rename(self.path_to_replace_complete.join(file_name.as_str()), self.path_to_repository.join(file_name.as_str())) {
                        unimplemented!();
                    }
                },
                FileOperation::Store(file_name) => {
                    
                    // Move from complete to storage
                    use crate::storage::LocalFileStorage;
                    let path_to_storage = self.path_to_repository.join(LocalFileStorage::DIRECTORY).join(file_name.as_str());
                    let path_to_complete_file = self.path_to_store_complete.join(file_name.as_str());
                    if let Err(error) = fs::rename(path_to_complete_file, path_to_storage.as_path()) {
                        unimplemented!();
                    }
                },
            }
        }
        Ok(())
    }

    // The fourth stage moves files that have been replaced into a location to be removed 
    fn process_fourth_stage(&self) -> Result<()> {
        // TODO: Ensure that all paths are resolved to UNC paths
        for job in self.atomic_jobs.as_slice() {
            match job {
                FileOperation::Create(_) => {
                    // Nothing to remove so no-op
                },
                FileOperation::Replace(file_name) => {
                    // Move from complete to current
                    if let Err(error) = fs::rename(self.path_to_replace_previous.join(file_name.as_str()), self.path_to_replace_remove.join(file_name.as_str())) {
                        unimplemented!();
                    }
                },
                FileOperation::Store(_) => {
                    // Nothing to remove so no op
                },
            }
        }
        Ok(())
    }

    // The fifth stage removes files that were replaced
    fn process_fifth_stage(&self) -> Result<()> {
        // TODO: Ensure that all paths are resolved to UNC paths
        for job in self.atomic_jobs.as_slice() {
            match job {
                FileOperation::Create(_) => {
                    // Nothing to remove so no-op
                },
                FileOperation::Replace(file_name) => {
                    // Remove the file that was replaced
                    if let Err(error) = fs::remove_file(self.path_to_replace_remove.join(file_name.as_str())) {
                        unimplemented!();
                    }
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
    use testspace::{TestSpace, TestSpaceFile};
    use crate::{AtomicUpdate, atomic::AtomicPaths};
    use super::{AtomicRecoveryJob, AtomicRecoveryList};
    use std::path;
    use std::fmt::{self, Display};
    // TODO: Test functions that simulate the various failure states

    impl<'a> Display for AtomicRecoveryList<'_> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                AtomicRecoveryList::Create(files) => {
                    let mut file_list = String::from("Files to create: ");
                    for file_name in files {
                        file_list.push_str(file_name);
                        file_list.push_str(", ");
                    }
                    writeln!(f, "{}", file_list)
                },
                AtomicRecoveryList::Store(files) => {
                    let mut file_list = String::from("Files to store: ");
                    for file_name in files {
                        file_list.push_str(file_name);
                        file_list.push_str(", ");
                    }
                    writeln!(f, "{}", file_list)
                },
                AtomicRecoveryList::Replace(files) => {
                    let mut file_list = String::from("Files to replace: ");
                    for file_name in files {
                        file_list.push_str(file_name);
                        file_list.push_str(", ");
                    }
                    writeln!(f, "{}", file_list)
                },
            }
        }
    }

    impl<'a> Display for AtomicRecoveryJob<'_> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                AtomicRecoveryJob::StageOne(jobs) => {
                    let mut job_list = String::from("Stage One Jobs\n");
                    for job in jobs {
                        let thing = format!("{}", job);
                        job_list.push_str(thing.as_str());
                    }
                    writeln!(f, "{}", job_list)
                },
                AtomicRecoveryJob::StageTwo(jobs) => {
                    let mut job_list = String::from("Stage Two Jobs\n");
                    for job in jobs {
                        let thing = format!("{}", job);
                        job_list.push_str(thing.as_str());
                    }
                    writeln!(f, "{}", job_list)
                },
                AtomicRecoveryJob::StageThree(jobs) => {
                    let mut job_list = String::from("Stage Three Jobs\n");
                    for job in jobs {
                        let thing = format!("{}", job);
                        job_list.push_str(thing.as_str());
                    }
                    writeln!(f, "{}", job_list)
                },
                AtomicRecoveryJob::StageFour(jobs) => {
                    let mut job_list = String::from("Stage Four Jobs\n");
                    for job in jobs {
                        let thing = format!("{}", job);
                        job_list.push_str(thing.as_str());
                    }
                    writeln!(f, "{}", job_list)
                },
                AtomicRecoveryJob::StageFive(jobs) => {
                    let mut job_list = String::from("Stage Five Jobs\n");
                    for job in jobs {
                        let thing = format!("{}", job);
                        job_list.push_str(thing.as_str());
                    }
                    writeln!(f, "{}", job_list)
                },
            }
            
        }
    }

     #[test]
    fn test_atomic_replace_init() {
        let ts = TestSpace::new();
        let repository_path = ts.get_path();
        AtomicUpdate::init(repository_path).expect("Failed to initialize atomic update");
        let temp_directory = repository_path.join(AtomicPaths::ReplaceWorking.get_path());
        let complete_directory = repository_path.join(AtomicPaths::ReplaceComplete.get_path());
        let previous_directory = repository_path.join(AtomicPaths::ReplacePrevious.get_path());
        let old_directory = repository_path.join(AtomicPaths::ReplaceRemove.get_path());
        assert!(temp_directory.exists());
        assert!(complete_directory.exists());
        assert!(previous_directory.exists());
        assert!(old_directory.exists());
    }

    #[test]
    fn test_atomic_create_init() {
        let ts = TestSpace::new();
        let repository_path = ts.get_path();
        AtomicUpdate::init(repository_path).expect("Failed to initialize atomic update");
        let complete_directory = repository_path.join(AtomicPaths::CreateComplete.get_path());
        let working_directory = repository_path.join(AtomicPaths::CreateWorking.get_path());
        assert!(working_directory.exists());
        assert!(complete_directory.exists());
    }

    #[test]
    fn test_atomic_store_init() {
        let ts = TestSpace::new();
        let repository_path = ts.get_path();
        AtomicUpdate::init(repository_path).expect("Failed to initialize atomic update");
        let complete_directory = repository_path.join(AtomicPaths::StoreComplete.get_path());
        let working_directory = repository_path.join(AtomicPaths::StoreWorking.get_path());
        assert!(working_directory.exists());
        assert!(complete_directory.exists());
    }

    #[test]
    fn test_first_stage_queue_create() {
        let mut ts = TestSpace::new();
        let repository_path = ts.get_path().to_path_buf();
        let mut atomic = AtomicUpdate::init(repository_path.as_path()).expect("Failed to initialize repository");
        for random_file in 0..5 {
            // Returns the path that we write data to
            let file_to_create = atomic.queue_create(format!("test{}",random_file));
            ts.create_file(file_to_create, 2048);
        }
        atomic.process_first_stage().expect("First Stage failed");
        

        for test_file in 0..5 {
            let path_to_test = repository_path.join(format!("{}\\test{}", AtomicPaths::CreateComplete.get_str(), test_file));
            println!("Testing for file {}", path_to_test.display());
            assert!(path_to_test.exists());
        }
    }

    #[test]
    fn test_third_stage_queue_create() {
        let mut ts = TestSpace::new();
        let repository_path = ts.get_path().to_path_buf();
        let mut atomic = AtomicUpdate::init(repository_path.as_path()).expect("Failed to initialize repository");
        for random_file in 0..5 {
            // Returns the path that we write data to
            let file_to_create = atomic.queue_create(format!("test{}",random_file));
            ts.create_file(file_to_create, 2048);
        }
        // Move from working to complete
        atomic.process_first_stage().expect("First Stage failed");
        // Does nothing
        atomic.process_second_stage().expect("Second Stage failed");
        // Move from complete to current
        atomic.process_third_stage().expect("Second Stage failed");
        for test_file in 0..5 {
            let path_to_test = repository_path.join(format!("test{}", test_file));
            println!("Testing for file {}", path_to_test.display());
            assert!(path_to_test.exists());
        }
    }

    #[test]
    fn test_first_stage_queue_replace() {
        let mut ts = TestSpace::new();
        let repository_path = ts.get_path().to_path_buf();
        let mut atomic = AtomicUpdate::init(repository_path.as_path()).expect("Failed to initialize repository");
        
        // Create the files that will be replaced
        for random_file in 0..5 {
            ts.create_file(repository_path.join(format!("test{}", random_file)), 4096);
        }
        // Create the files that will replace the files in repository
        for random_file in 0..5 {
            // Returns the path that we write data to
            let file_to_create = atomic.queue_replace(format!("test{}",random_file));
            ts.create_file(file_to_create, 2048);
        }
        // First stage only moves from working to complete
        atomic.process_first_stage().expect("First Stage failed");
        for test_file in 0..5 {
            let path_to_test = repository_path.join(format!("{}\\test{}", AtomicPaths::ReplaceComplete.get_str(), test_file));
            println!("Testing for file {}", path_to_test.display());
            assert!(path_to_test.exists());
        }
    }

    #[test]
    fn test_second_stage_queue_replace() {
        let mut ts = TestSpace::new();
        let repository_path = ts.get_path().to_path_buf();
        let mut atomic = AtomicUpdate::init(repository_path.as_path()).expect("Failed to initialize repository");
        
        // Create the files that will be replaced
        for random_file in 0..5 {
            ts.create_file(repository_path.join(format!("test{}", random_file)), 4096);
        }
        // Create the files that will replace the files in repository
        for random_file in 0..5 {
            // Returns the path that we write data to
            let file_to_create = atomic.queue_replace(format!("test{}",random_file));
            ts.create_file(file_to_create, 2048);
        }
        // First stage only moves from working to complete
        atomic.process_first_stage().expect("First Stage failed");
        // Second stage moves from current to previous
        atomic.process_second_stage().expect("Second Stage failed");

        for test_file in 0..5 {
            let path_to_test = repository_path.join(format!("{}\\test{}", AtomicPaths::ReplacePrevious.get_str(), test_file));
            println!("Testing for file {}", path_to_test.display());
            assert!(path_to_test.exists());
        }
    }

    #[test]
    fn test_third_stage_queue_replace() {
        let mut ts = TestSpace::new();
        let repository_path = ts.get_path().to_path_buf();
        let mut atomic = AtomicUpdate::init(repository_path.as_path()).expect("Failed to initialize repository");
        
        // Create the files that will be replaced
        for random_file in 0..5 {
            ts.create_file(repository_path.join(format!("test{}", random_file)), 4096);
        }
        // Create the files that will replace the files in repository
        for random_file in 0..5 {
            // Returns the path that we write data to
            let file_to_create = atomic.queue_replace(format!("test{}",random_file));
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
        // Files are back in repository
        for test_file in 0..5 {
            let path_to_test = repository_path.join(format!("test{}", test_file));
            println!("Testing for file {}", path_to_test.display());
            assert!(path_to_test.exists());
        }
    }

    #[test]
    fn test_fourth_stage_queue_replace() {
        let mut ts = TestSpace::new();
        let repository_path = ts.get_path().to_path_buf();
        let mut atomic = AtomicUpdate::init(repository_path.as_path()).expect("Failed to initialize repository");
        
        // Create the files that will be replaced
        for random_file in 0..5 {
            ts.create_file(repository_path.join(format!("test{}", random_file)), 4096);
        }
        // Create the files that will replace the files in repository
        for random_file in 0..5 {
            // Returns the path that we write data to
            let file_to_create = atomic.queue_replace(format!("test{}",random_file));
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
        // Files are back in repository
        for test_file in 0..5 {
            let path_to_test = repository_path.join(format!("{}\\test{}", AtomicPaths::ReplaceRemove.get_str(), test_file));
            println!("Testing for file {}", path_to_test.display());
            assert!(path_to_test.exists());
        }
    }

    #[test]
    fn test_fifth_stage_queue_replace() {
        let mut ts = TestSpace::new();
        let repository_path = ts.get_path().to_path_buf();
        let mut atomic = AtomicUpdate::init(repository_path.as_path()).expect("Failed to initialize repository");
        
        // Create the files that will be replaced
        for random_file in 0..5 {
            ts.create_file(repository_path.join(format!("test{}", random_file)), 4096);
        }
        // Create the files that will replace the files in repository
        for random_file in 0..5 {
            // Returns the path that we write data to
            let file_to_create = atomic.queue_replace(format!("test{}",random_file));
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
            let path_to_test = repository_path.join(format!("{}\\test{}", AtomicPaths::ReplaceRemove.get_str(), test_file));
            println!("Testing for file {}", path_to_test.display());
            assert_eq!(path_to_test.exists(), false);
        }
    }

    #[test]
    fn test_atomic_complete() {
        let mut ts = TestSpace::new();
        let repository_path = ts.get_path().to_path_buf();
        let mut atomic = AtomicUpdate::init(repository_path.as_path()).expect("Failed to initialize repository");
        // Create the files that will be replaced
        for random_file in 0..5 {
            ts.create_file(repository_path.join(format!("test{}", random_file)), 4096);
        }
        // Create the files that will replace the files in repository
        for random_file in 0..5 {
            // Returns the path that we write data to
            let file_to_create = atomic.queue_replace(format!("test{}", random_file));
            ts.create_file(file_to_create, 2048);
        }
        // Create the files that will be created in the repository
        for random_file in 0..5 {
            // Returns the path that we write data to
            let file_to_create = atomic.queue_create(format!("test_create{}", random_file));
            ts.create_file(file_to_create, 2048);
        }
        atomic.complete().expect("Atomic operation failed");
        // Check for created files
        for random_file in 0..5 {
            let file_to_create = repository_path.join(format!("test_create{}", random_file));
            assert!(file_to_create.exists());
        }
        // Check for replaced files
        for random_file in 0..5 {
            let file_to_create = repository_path.join(format!("test{}", random_file));
            assert!(file_to_create.exists());
        }
    }

    #[test]
    fn test_atomic_store() {
        use crate::storage::LocalFileStorage;
        let mut ts = TestSpace::new();
        let repository_path = ts.get_path().to_path_buf();
        let mut atomic = AtomicUpdate::init(repository_path.as_path()).expect("Failed to initialize repository");
        let fs = LocalFileStorage::initialize(repository_path.as_path()).expect("Failed to init file storage");
        // Create the files that will be stored
        for random_file in 0..5 {
            ts.create_file(repository_path.join(format!("test{}", random_file)), 4096);
            let file_to_store = repository_path.join(format!("test{}", random_file));
            let place_to_store = atomic.queue_store(format!("test{}", random_file));
            let error_string = format!("Failed to store file, from {} to {}", file_to_store.display(), place_to_store.display());
            fs.store_file(file_to_store.as_path(), place_to_store.as_path()).expect(error_string.as_str());
        }
        // Process stage one
        atomic.process_first_stage().expect("Failed first stage");
        // All files should have been moved to storage complete
        for file in 0..5 {
            let path_to_check = atomic.path_to_store_complete.join(format!("test{}", file));
            assert!(path_to_check.exists());
        }
        atomic.process_third_stage().expect("All files should have been moved to storage");
        // process stage three - Files should be in the storage folder in the repository
        let storage_path = repository_path.join(LocalFileStorage::DIRECTORY);
        for file in 0..5 {
            let path_to_check = storage_path.join(format!("test{}", file));
            assert!(path_to_check.exists());
        }
    }

    #[test]
    fn was_interrupted_test() {
        fn build_atomic(path_to_repository: &path::Path) -> AtomicUpdate {
            let atomic = AtomicUpdate::init(path_to_repository).expect("Failed to initialize Atomic");
            atomic
        }
        let mut ts = TestSpace::new().allow_cleanup(false);
        let atomic = build_atomic(ts.get_path());
        
        let test_dir = atomic.path_to_store_working;
        println!("Current test path is {}", test_dir.display());
        ts.create_file(test_dir.join("test_file.c"), 1024);
        assert_eq!(AtomicUpdate::was_interrupted(ts.get_path()), true);
    }

    #[test]
    fn contains_files_test() {
        let mut ts = TestSpace::new();
        let files_created = ts.create_random_files(2, 1024);
        let path_with_files = ts.get_path();
        let result = AtomicUpdate::contains_files(path_with_files);
        assert_eq!(result, true);
        let mut ts2 = TestSpace::new();
        let empty_path = ts2.get_path();
        let result2 = AtomicUpdate::contains_files(empty_path);
        assert_eq!(result2, false);
    }

    #[test]
    fn stage_one_recovery_test() {
        // Initialize AtomicUpdate and set up paths 
        let mut ts = TestSpace::new().allow_cleanup(true);
        
        let repository_path = ts.get_path().to_path_buf();
        let mut atomic = AtomicUpdate::init(repository_path.as_path()).expect("Failed to initialize repository");
        // Create operation of 5 files interrupted during stage one after 2 files
        let cw_file_list = vec!("a","b","c");
        let cc_file_list = vec!("d","e");
        
        ts.create_files("cw", cw_file_list.as_slice(), 1024);
        ts.create_files("cc", cc_file_list.as_slice(), 1024);
        
        let stage_one_jobs = AtomicUpdate::create_stage_one_list(cw_file_list.clone(), vec!(), vec!());
        let create_jobs = stage_one_jobs.get_create_jobs();
        let store_jobs = stage_one_jobs.get_store_jobs();
        assert_eq!(create_jobs, Some(cw_file_list.as_slice()));
        assert_eq!(store_jobs, None);
    }

    #[test]
    fn stage_two_recovery_test() {
        // Stage two moves files from the repository to previous
        let mut ts = TestSpace::new().allow_cleanup(true);
        let repository_path = ts.get_path().to_path_buf();
        let mut atomic = AtomicUpdate::init(repository_path.as_path()).expect("Failed to initialize repository");
        // 3 files being processed, operation is interrupted during stage 2 after 1 file
        // 3 files in complete as stage 1 was completed
        let rc_file_list = vec!("a","b","c");
        let rp_file_list = vec!("c");
        ts.create_files("rc", rc_file_list.as_slice(), 1024);
        // 1 file in previous
        ts.create_files("rp", rp_file_list.as_slice(), 1024);
        let stage_two_files = AtomicUpdate::create_stage_two_list(rp_file_list, rc_file_list, vec!());
        println!("{}", stage_two_files);
        // We expect to need to move a and b from the repository to rp to recover this stage
        let expected_result = vec!("a","b");
        assert_eq!(stage_two_files.get_replace_jobs(), Some(expected_result.as_slice()));
    }

    #[test]
    fn stage_two_recovery_test_2() {
        // Stage two moves files from the repository to previous
        let mut ts = TestSpace::new().allow_cleanup(true);
        let repository_path = ts.get_path().to_path_buf();
        let mut atomic = AtomicUpdate::init(repository_path.as_path()).expect("Failed to initialize repository");
        // 4 files in atomic, operation is interrupted during stage 1 after 2 files, this means stage 2 wasn't started
        // 2 files in complete
        let rc_file_list = vec!("a","b");
        // 2 file in working
        let rw_file_list = vec!("c","d");
        // 0 files in previous
        let rp_file_list = vec!();
        ts.create_files("rc", rc_file_list.as_slice(), 1024);
        // 1 file in previous
        ts.create_files("rw", rw_file_list.as_slice(), 1024);
        let stage_two_files = AtomicUpdate::create_stage_two_list(rp_file_list, rc_file_list, rw_file_list);
        println!("{}", stage_two_files);
        // We expect to need to move a, b, c and d from the repository to rp to recover this stage
        let expected_result = vec!("a","b","c","d");
        assert_eq!(stage_two_files.get_replace_jobs(), Some(expected_result.as_slice()));
    }

    #[test]
    fn stage_three_recovery_test() {
        let mut ts = TestSpace::new().allow_cleanup(true);
        let repository_path = ts.get_path().to_path_buf();
        let mut atomic = AtomicUpdate::init(repository_path.as_path()).expect("Failed to initialize repository");
        // 3 files in atomic, one in each, operation interrupted during stage one, one file completed
        // 1 file completed
        let sc_file_list = vec!("a");
        // 2 file in working
        let rw_file_list = vec!("b");
        let cw_file_list = vec!("c");
        // Create the files
        ts.create_files("rw", rw_file_list.as_slice(), 1024);
        ts.create_files("cw", cw_file_list.as_slice(), 1024);
        ts.create_files("sc", sc_file_list.as_slice(), 1024);
        
        let stage_three_files = AtomicUpdate::create_stage_three_list(rw_file_list, vec!(), vec!(), sc_file_list, cw_file_list, vec!());
        println!("{}", stage_three_files);
        let expected_create = vec!("c");
        let expected_store = vec!("a");
        let expected_replace = vec!("b");
        assert_eq!(stage_three_files.get_create_jobs(), Some(expected_create.as_slice()));
        assert_eq!(stage_three_files.get_store_jobs(), Some(expected_store.as_slice()));
        assert_eq!(stage_three_files.get_replace_jobs(), Some(expected_replace.as_slice()));
    }

    #[test]
    fn stage_four_recovery_test() {
        // Previous to remove
        let mut ts = TestSpace::new().allow_cleanup(true);
        let repository_path = ts.get_path().to_path_buf();
        let mut atomic = AtomicUpdate::init(repository_path.as_path()).expect("Failed to initialize repository");
        // 5 files in atomic, interrupted during stage 2 after one operation
        // 1 file completed
        let rp_file_list = vec!("a");
        // 2 file in working
        let rc_file_list = vec!("b","c");
        // Create the files
        ts.create_files("rp", rp_file_list.as_slice(), 1024);
        ts.create_files("rc", rc_file_list.as_slice(), 1024);

        // expect a,b,c as result since the interruption happened before stage four commenced
        // expect None for all other job types other than replace
    }
}