enum AtomicRecoveryList<'a> {
    Create(Vec<&'a str>),
    Store(Vec<&'a str>),
    Replace(Vec<&'a str>),
}
struct AtomicRecovery;

impl AtomicRecovery {

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
    // TODO: This probably doesn't belong in this module
    // pub fn was_interrupted(path_to_repository: &path::Path) -> bool {
    //     // Construct paths
    //     let cw = path_to_repository.join(AtomicLocation::CreateWorking.get_path());
    //     let cc = path_to_repository.join(AtomicLocation::CreateComplete.get_path());
    //     let sw = path_to_repository.join(AtomicLocation::StoreWorking.get_path());
    //     let sc = path_to_repository.join(AtomicLocation::StoreComplete.get_path());
    //     let rw = path_to_repository.join(AtomicLocation::ReplaceWorking.get_path());
    //     let rc = path_to_repository.join(AtomicLocation::ReplaceComplete.get_path());
    //     let rp = path_to_repository.join(AtomicLocation::ReplacePrevious.get_path());
    //     let rr = path_to_repository.join(AtomicLocation::ReplaceRemove.get_path());
    //     // Check for any files in the above paths
    //     AtomicUpdate::contains_files(cw.as_path()) || AtomicUpdate::contains_files(cc.as_path()) || AtomicUpdate::contains_files(sw.as_path()) || AtomicUpdate::contains_files(sc.as_path()) || 
    //         AtomicUpdate::contains_files(rw.as_path()) || AtomicUpdate::contains_files(rc.as_path()) || AtomicUpdate::contains_files(rp.as_path()) || AtomicUpdate::contains_files(rr.as_path())
    // }

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

    // pub fn recover(path_to_repository: &path::Path) -> Result<(), Error> {
    //     // Construct paths
    //     let cw = path_to_repository.join(AtomicLocation::CreateWorking.get_path());
    //     let cc = path_to_repository.join(AtomicLocation::CreateComplete.get_path());
    //     let sw = path_to_repository.join(AtomicLocation::StoreWorking.get_path());
    //     let sc = path_to_repository.join(AtomicLocation::StoreComplete.get_path());
    //     let rw = path_to_repository.join(AtomicLocation::ReplaceWorking.get_path());
    //     let rc = path_to_repository.join(AtomicLocation::ReplaceComplete.get_path());
    //     let rp = path_to_repository.join(AtomicLocation::ReplacePrevious.get_path());
    //     let rr = path_to_repository.join(AtomicLocation::ReplaceRemove.get_path());

    //     // TODO: If get_files is called elsewhere then some context may need to be added
    //     let cw_list = AtomicUpdate::get_files(cw.as_path())?;
    //     let cc_list = AtomicUpdate::get_files(cc.as_path())?;
    //     let sw_list = AtomicUpdate::get_files(sw.as_path())?;
    //     let sc_list = AtomicUpdate::get_files(sc.as_path())?;
    //     let rw_list = AtomicUpdate::get_files(rw.as_path())?;
    //     let rc_list = AtomicUpdate::get_files(rc.as_path())?;
    //     let rp_list = AtomicUpdate::get_files(rp.as_path())?;
    //     let rr_list = AtomicUpdate::get_files(rr.as_path())?;
    //     // First can we restore?
    //     let restore_possible = AtomicUpdate::can_restore(sw_list.len(), sc_list.len(), cw_list.len(), cc_list.len(), rw_list.len(), rp_list.len());
    //     if restore_possible {
            
    //         // let jobs_to_execute = AtomicUpdate::build_recovery_job();
            
            

    //         // Execute recovery jobs
    //     } else {
    //         // Remove all files in all atomic paths and report that operation did not complete
    //         // This is a dangerous operation and must be cleaned in the reverse order of stages to ensure we dont trick recoverer into 
    //         // thinking that an operation was more successful than it was
    //         // TODO: We should only need to clean the working directories since a single stage is completed one at a time
    //         AtomicUpdate::clean_directory(rp.as_path())?;
    //         AtomicUpdate::clean_directory(rr.as_path())?;
    //         AtomicUpdate::clean_directory(cc.as_path())?;
    //         AtomicUpdate::clean_directory(sc.as_path())?;
    //         AtomicUpdate::clean_directory(rc.as_path())?;
    //         AtomicUpdate::clean_directory(cw.as_path())?;
    //         AtomicUpdate::clean_directory(sw.as_path())?;
    //         AtomicUpdate::clean_directory(rw.as_path())?;
    //     }
    //     Ok(())
    // }

    // fn build_recovery_job(cw_list: Vec<String>, cc_list: Vec<String>, sc_list: Vec<String>, sw_list: Vec<String>, rp_list: Vec<String>, rw_list: Vec<String>, rc_list: Vec<String>, rr_list: Vec<String>) -> Vec<AtomicRecoveryJob<'a>> {
    //     // TODO: Lists can be partial, for instance 4 items in working, 3 items in previous means 7 items for complete
    //     // Files for Stage two create, store and replace
    //     let mut rp_ref_list: Vec<&str> = rp_list.iter().map(|s| s.as_str()).collect();
    //     let mut rw_ref_list: Vec<&str> = rw_list.iter().map(|s| s.as_str()).collect();
    //     let mut rc_ref_list: Vec<&str> = rc_list.iter().map(|s| s.as_str()).collect();
    //     let mut rr_ref_list: Vec<&str> = rr_list.iter().map(|s| s.as_str()).collect();
    //     let mut sw_ref_list: Vec<&str> = sw_list.iter().map(|s| s.as_str()).collect();
    //     let mut sc_ref_list: Vec<&str> = sc_list.iter().map(|s| s.as_str()).collect();
    //     let mut cw_ref_list: Vec<&str> = cw_list.iter().map(|s| s.as_str()).collect();
    //     let mut cc_ref_list: Vec<&str> = cc_list.iter().map(|s| s.as_str()).collect();
        
        
    //     // let mut stage_three_jobs = Vec::new();
    //     // let mut stage_four_jobs = Vec::new();
    //     // let mut stage_five_jobs = Vec::new();
    //     // Stage One still needs to be performed as it may have only partially been completed - ie move all working to completed
    //     let stage_one_jobs = AtomicUpdate::create_stage_one_list(cw_ref_list.clone(), sw_ref_list.clone(), rw_ref_list.clone());
    //     let stage_two_jobs = AtomicUpdate::create_stage_two_list(rp_ref_list.clone(), rc_ref_list.clone(), rw_ref_list.clone());
    //     let stage_three_jobs = AtomicUpdate::create_stage_three_list(rw_ref_list.clone(), rc_ref_list.clone(), sw_ref_list.clone(), sc_ref_list.clone(), cw_ref_list.clone(), cc_ref_list.clone());
        

    //     // Files for stage four create store and replace
    //     // Files for stage five create store and replace
    //     Vec::new()
    // }

    // fn create_stage_three_list(mut rw_list: Vec<&'a str>, mut rc_list: Vec<&'a str>, mut sw_list: Vec<&'a str>, mut sc_list: Vec<&'a str>, mut cw_list: Vec<&'a str>, mut cc_list: Vec<&'a str>) -> AtomicRecoveryJob<'a> {
    //     // Move all the files in completed to the repository
    //     sc_list.append(&mut sw_list);
    //     cc_list.append(&mut cw_list);
    //     rc_list.append(&mut rw_list);
    //     let mut stage_three_jobs = Vec::new();
    //     let stage_two_create = AtomicRecoveryList::Create(cc_list);
    //     let stage_two_store = AtomicRecoveryList::Store(sc_list);
    //     let stage_two_replace = AtomicRecoveryList::Replace(rc_list);
    //     stage_three_jobs.push(stage_two_create);
    //     stage_three_jobs.push(stage_two_store);
    //     stage_three_jobs.push(stage_two_replace);
    //     AtomicRecoveryJob::StageThree(stage_three_jobs)
    // }

    /// Create file list for stage two
    // fn create_stage_two_list(rp_list: Vec<&'a str>, mut rc_list: Vec<&'a str>, mut rw_list: Vec<&'a str>) -> AtomicRecoveryJob<'a> {
    //     /// Subtracts a vector from another vector modifying the resultant vector in place
    //     fn subtract_vector(vector: &mut Vec<&str>, vector_to_subtract: Vec<&str>) {
    //         let mut i = 0;
    //         while i != vector.len() {
    //             if vector_to_subtract.contains(&vector[i]) {
    //                 vector.remove(i);
    //             } else {
    //                 i += 1;
    //             }
    //         }
    //     }
    //     // Move files from repository to previous - The complete list for files can be found from (complete + working) - files already in previous, if working contains files then there should be no files in previous
    //     // TODO: Assuming that working is empty here may not be correct, since if some files are moved to complete but not all then recovery is still possible but working is not empty
    //     rc_list.append(&mut rw_list);
    //     println!("Appended List: {:?}", rc_list);
    //     subtract_vector(&mut rc_list, rp_list.clone());
    //     println!("Subtracted Result: {:?} - {:?} = {:?}", rc_list, rp_list, rc_list.clone());
    //     //rc_list.drain_filter(|file_name| rp_list.contains(file_name));
    //     // TODO: Working must be empty if previous has files
    //     let mut stage_two_jobs = Vec::new();
    //     let stage_two_replace = AtomicRecoveryList::Replace(rc_list);
    //     stage_two_jobs.push(stage_two_replace);
    //     AtomicRecoveryJob::StageTwo(stage_two_jobs)
    // }

    /// Create file list for stage one
    // fn create_stage_one_list(cw_list: Vec<&'a str>, sw_list: Vec<&'a str>, rw_list: Vec<&'a str>) -> AtomicRecoveryJob<'a> {
    //     let mut stage_one_jobs = Vec::new();
    //     let stage_one_create = AtomicRecoveryList::Create(cw_list);
    //     let stage_one_store = AtomicRecoveryList::Store(sw_list);
    //     let stage_one_replace = AtomicRecoveryList::Replace(rw_list);
    //     stage_one_jobs.push(stage_one_create);
    //     stage_one_jobs.push(stage_one_store);
    //     stage_one_jobs.push(stage_one_replace);
    //     AtomicRecoveryJob::StageOne(stage_one_jobs)
    // }

    // fn clean_directory(path_to_directory: &path::Path) -> Result<(), Error> {
    //     let items_found = match fs::read_dir(path_to_directory) {
    //         Ok(result) => result,
    //         Err(error) => unimplemented!(),
    //     };
    //     for item in items_found {
    //         let valid_item = match item {
    //             Ok(valid_item) => valid_item,
    //             Err(error) => unimplemented!(),
    //         };
    //         match valid_item.file_type() {
    //             Ok(file_type) if file_type.is_file() => {
    //                 if let Err(error) = fs::remove_file(valid_item.path()) {
    //                     unimplemented!();
    //                 }
    //             },
    //             Ok(file_type) => {
    //                 // Unknown item type
    //             },
    //             Err(error) => unimplemented!(),
    //         }
    //     }
    //     Ok(())
    // }

    // /// Get a list of file names for the given directory
    // fn get_files(path_to_directory: &path::Path) -> Result<Vec<String>, Error> {
    //     let mut results = Vec::new();
    //     let items_found = match fs::read_dir(path_to_directory) {
    //         Ok(result) => result,
    //         Err(error) => unimplemented!(),
    //     };
    //     for item in items_found {
    //         let valid_item = match item {
    //             Ok(valid_item) => valid_item,
    //             Err(error) => unimplemented!(),
    //         };
    //         match valid_item.file_type() {
    //             Ok(file_type) if file_type.is_file() => {
    //                 // let file_name = String::from(valid_item.path().file_name().unwrap().to_str().unwrap());
    //                 let file_name = match valid_item.path().file_name() {
    //                     Some(os_name) => {
    //                         match os_name.to_str() {
    //                             Some(file_name) => String::from(file_name),
    //                             None => unimplemented!(), // Invalid UTF8
    //                         }
    //                     },
    //                     None => unimplemented!(), // No file name
    //                 };
    //                 results.push(file_name);
    //             },
    //             Ok(file_type) => {
    //                 // Unknown item type
    //             },
    //             Err(error) => unimplemented!(),
    //         }
    //     }
    //     Ok(results)
    // }
}
enum AtomicRecoveryJob<'a> {
    StageOne(Vec<AtomicRecoveryList<'a>>),
    StageTwo(Vec<AtomicRecoveryList<'a>>),
    StageThree(Vec<AtomicRecoveryList<'a>>),
    StageFour(Vec<AtomicRecoveryList<'a>>),
    StageFive(Vec<AtomicRecoveryList<'a>>),
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

// impl std::fmt::Display for FileOperation {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         match self {
//             FileOperation::Create(_) => write!(f, "FileOperation::Create"),
//             FileOperation::Replace(_) => write!(f, "FileOperation::Replace"),
//             FileOperation::Store(_) => write!(f, "FileOperation::Store"),
//         }
//     }
// }

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

#[cfg(test)]
mod tests {
    use testspace::TestSpace;
    use super::{AtomicRecovery, AtomicLocation};
    
    use std::path;
    use std::fmt::{self, Display};

    // TODO: Test functions that simulate the various failure states

    // #[test]
    // fn was_interrupted_test() {
    //     fn build_atomic(path_to_repository: &path::Path) -> AtomicUpdate {
    //         let atomic = AtomicUpdate::init(path_to_repository).expect("Failed to initialize Atomic");
    //         atomic
    //     }
    //     let mut ts = TestSpace::new().allow_cleanup(false);
    //     let atomic = build_atomic(ts.get_path());
        
    //     let test_dir = atomic.path_to_store_working;
    //     println!("Current test path is {}", test_dir.display());
    //     ts.create_file(test_dir.join("test_file.c"), 1024);
    //     assert_eq!(AtomicUpdate::was_interrupted(ts.get_path()), true);
    // }

    // #[test]
    // fn contains_files_test() {
    //     let mut ts = TestSpace::new();
    //     let files_created = ts.create_random_files(2, 1024);
    //     let path_with_files = ts.get_path();
    //     let result = AtomicUpdate::contains_files(path_with_files);
    //     assert_eq!(result, true);
    //     let mut ts2 = TestSpace::new();
    //     let empty_path = ts2.get_path();
    //     let result2 = AtomicUpdate::contains_files(empty_path);
    //     assert_eq!(result2, false);
    // }

    // #[test]
    // fn stage_one_recovery_test() {
    //     // Initialize AtomicUpdate and set up paths 
    //     let mut ts = TestSpace::new().allow_cleanup(true);
        
    //     let repository_path = ts.get_path().to_path_buf();
    //     let mut atomic = AtomicUpdate::init(repository_path.as_path()).expect("Failed to initialize repository");
    //     // Create operation of 5 files interrupted during stage one after 2 files
    //     let cw_file_list = vec!("a","b","c");
    //     let cc_file_list = vec!("d","e");
        
    //     ts.create_files("cw", cw_file_list.as_slice(), 1024);
    //     ts.create_files("cc", cc_file_list.as_slice(), 1024);
        
    //     let stage_one_jobs = AtomicUpdate::create_stage_one_list(cw_file_list.clone(), vec!(), vec!());
    //     let create_jobs = stage_one_jobs.get_create_jobs();
    //     let store_jobs = stage_one_jobs.get_store_jobs();
    //     assert_eq!(create_jobs, Some(cw_file_list.as_slice()));
    //     assert_eq!(store_jobs, None);
    // }

    // #[test]
    // fn stage_two_recovery_test() {
    //     // Stage two moves files from the repository to previous
    //     let mut ts = TestSpace::new().allow_cleanup(true);
    //     let repository_path = ts.get_path().to_path_buf();
    //     let mut atomic = AtomicUpdate::init(repository_path.as_path()).expect("Failed to initialize repository");
    //     // 3 files being processed, operation is interrupted during stage 2 after 1 file
    //     // 3 files in complete as stage 1 was completed
    //     let rc_file_list = vec!("a","b","c");
    //     let rp_file_list = vec!("c");
    //     ts.create_files("rc", rc_file_list.as_slice(), 1024);
    //     // 1 file in previous
    //     ts.create_files("rp", rp_file_list.as_slice(), 1024);
    //     let stage_two_files = AtomicUpdate::create_stage_two_list(rp_file_list, rc_file_list, vec!());
    //     println!("{}", stage_two_files);
    //     // We expect to need to move a and b from the repository to rp to recover this stage
    //     let expected_result = vec!("a","b");
    //     assert_eq!(stage_two_files.get_replace_jobs(), Some(expected_result.as_slice()));
    // }

    // #[test]
    // fn stage_two_recovery_test_2() {
    //     // Stage two moves files from the repository to previous
    //     let mut ts = TestSpace::new().allow_cleanup(true);
    //     let repository_path = ts.get_path().to_path_buf();
    //     let mut atomic = AtomicUpdate::init(repository_path.as_path()).expect("Failed to initialize repository");
    //     // 4 files in atomic, operation is interrupted during stage 1 after 2 files, this means stage 2 wasn't started
    //     // 2 files in complete
    //     let rc_file_list = vec!("a","b");
    //     // 2 file in working
    //     let rw_file_list = vec!("c","d");
    //     // 0 files in previous
    //     let rp_file_list = vec!();
    //     ts.create_files("rc", rc_file_list.as_slice(), 1024);
    //     // 1 file in previous
    //     ts.create_files("rw", rw_file_list.as_slice(), 1024);
    //     let stage_two_files = AtomicUpdate::create_stage_two_list(rp_file_list, rc_file_list, rw_file_list);
    //     println!("{}", stage_two_files);
    //     // We expect to need to move a, b, c and d from the repository to rp to recover this stage
    //     let expected_result = vec!("a","b","c","d");
    //     assert_eq!(stage_two_files.get_replace_jobs(), Some(expected_result.as_slice()));
    // }

    // #[test]
    // fn stage_three_recovery_test() {
    //     let mut ts = TestSpace::new().allow_cleanup(true);
    //     let repository_path = ts.get_path().to_path_buf();
    //     let mut atomic = AtomicUpdate::init(repository_path.as_path()).expect("Failed to initialize repository");
    //     // 3 files in atomic, one in each, operation interrupted during stage one, one file completed
    //     // 1 file completed
    //     let sc_file_list = vec!("a");
    //     // 2 file in working
    //     let rw_file_list = vec!("b");
    //     let cw_file_list = vec!("c");
    //     // Create the files
    //     ts.create_files("rw", rw_file_list.as_slice(), 1024);
    //     ts.create_files("cw", cw_file_list.as_slice(), 1024);
    //     ts.create_files("sc", sc_file_list.as_slice(), 1024);
        
    //     let stage_three_files = AtomicUpdate::create_stage_three_list(rw_file_list, vec!(), vec!(), sc_file_list, cw_file_list, vec!());
    //     println!("{}", stage_three_files);
    //     let expected_create = vec!("c");
    //     let expected_store = vec!("a");
    //     let expected_replace = vec!("b");
    //     assert_eq!(stage_three_files.get_create_jobs(), Some(expected_create.as_slice()));
    //     assert_eq!(stage_three_files.get_store_jobs(), Some(expected_store.as_slice()));
    //     assert_eq!(stage_three_files.get_replace_jobs(), Some(expected_replace.as_slice()));
    // }

    // #[test]
    // fn stage_four_recovery_test() {
    //     // Previous to remove
    //     let mut ts = TestSpace::new().allow_cleanup(true);
    //     let repository_path = ts.get_path().to_path_buf();
    //     let mut atomic = AtomicUpdate::init(repository_path.as_path()).expect("Failed to initialize repository");
    //     // 5 files in atomic, interrupted during stage 2 after one operation
    //     // 1 file completed
    //     let rp_file_list = vec!("a");
    //     // 2 file in working
    //     let rc_file_list = vec!("b","c");
    //     // Create the files
    //     ts.create_files("rp", rp_file_list.as_slice(), 1024);
    //     ts.create_files("rc", rc_file_list.as_slice(), 1024);

    //     // expect a,b,c as result since the interruption happened before stage four commenced
    //     // expect None for all other job types other than replace
    // }
}