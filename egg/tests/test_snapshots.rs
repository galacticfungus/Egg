use testspace::TestSpace;
use egg;

// TODO: Extensive path testing ie various user inputs

#[test]
fn egg_initialize_a_simple_repository_test() {
    // Since ts always returns absolute paths
    let ts = TestSpace::new();
    let working_path = ts.get_path().canonicalize().unwrap();
    let egg = egg::Repository::create_repository(working_path.as_path()).expect("Failed to create repository");
    let loaded_egg = egg::Repository::find_egg(working_path.as_path()).expect("Failed to find egg");
    // assert_eq!(egg, loaded_egg);
    assert_eq!(loaded_egg.get_working_path(), working_path.as_path());
}

#[test]
fn egg_taking_simple_snapshot_test() {
    let ts = TestSpace::new();
    let mut ts2 = ts.create_child();
    let files_to_snapshot = ts2.create_random_files(2, 4096);
    let working_path = ts.get_path();
    let mut repo = egg::Repository::create_repository(working_path).expect("Failed to create repository");
    println!("working: {}", working_path.display());
    for file in &files_to_snapshot {
        println!("file: {}", file.display());
    }
    let snapshot_id = repo.take_snapshot(None, "Test Message", files_to_snapshot.clone()).expect("Failed to take snapshot");
    let snapshot = repo.get_snapshot(snapshot_id).expect("Could not retrieve the snapshot that was just taken");
    let files_snapshotted = snapshot.get_files();
    for (index, file) in files_snapshotted.iter().enumerate() {
        assert_eq!(file.filesize(), 4096);
        let original_absolute = files_to_snapshot[index].canonicalize().expect("Failed to convert test path to an absolute path");
        assert_eq!(original_absolute.as_path(), file.path());
    }
}

#[test]
fn egg_get_staged_files_test() {
    
}

#[test]
fn egg_get_recent_snapshot_test() {
    let ts = TestSpace::new();
    let mut ts2 = ts.create_child();
    let files_to_snapshot = ts2.create_random_files(2, 4096);
    let working_path = ts.get_path();
    let mut repo = egg::Repository::create_repository(working_path).expect("Failed to create repository");
    println!("working: {}", working_path.display());
    for file in &files_to_snapshot {
        println!("file: {}", file.display());
    }
    let snapshot_id = repo.take_snapshot(None, "Test Message", files_to_snapshot.clone()).expect("Failed to take snapshot");
    let latest_snapshot = repo.get_latest_snapshot().expect("Failed to retrieve latest snapshot");
    match latest_snapshot {
        Some(latest_snapshot) => assert_eq!(latest_snapshot, snapshot_id),
        None => panic!("No latest snapshot was found")
    }
    
}

#[test]
fn egg_take_snapshot_with_child_test() {
    let ts = TestSpace::new().allow_cleanup(false);
    let mut ts2 = ts.create_child();
    let mut files_to_snapshot = ts2.create_random_files(2, 4096);
    let working_path = ts.get_path();
    let mut repo = egg::Repository::create_repository(working_path).expect("Failed to create repository");
    println!("working: {}", working_path.display());
    for file in &files_to_snapshot {
        println!("file: {}", file.display());
    }
    let snapshot_id = repo.take_snapshot(None, "Test Message", files_to_snapshot.clone()).expect("Failed to take snapshot");
    let mut new_files = ts2.create_random_files(2, 2048);
    files_to_snapshot.append(&mut new_files);
    let child_id = repo.take_snapshot(Some(snapshot_id), "A child snapshot", files_to_snapshot).expect("Failed to get child snapshot");
    let child_snapshot = repo.get_snapshot(child_id).expect("Child not found");
    for file in child_snapshot.get_files() {
        println!("{:?}", file.path());
    }
}
