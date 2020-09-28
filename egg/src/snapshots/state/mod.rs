use super::SnapshotsState;

mod builder;
mod change;
mod init;

// Helper struct to help with managing changes to state
pub struct StateBuilder<'a> {
    state: &'a mut SnapshotsState, // The state to modify
}

#[cfg(test)]
mod tests {
    use crate::hash::Hash;
    use crate::snapshots::SnapshotsState;
    use crate::AtomicUpdate;
    use testspace::TestSpace;

    impl PartialEq for SnapshotsState {
        fn eq(&self, other: &Self) -> bool {
            if self.working_snapshot != other.working_snapshot {
                return false;
            }
            if self.latest_snapshot != other.latest_snapshot {
                return false;
            }
            return true;
        }
    }

    #[test]
    fn test_create_new_state() {
        // Tests that the snapshot state file is correctly initialized and loaded
        let mut ts = TestSpace::new();
        ts.create_dir("snapshots");
        let base_path = ts.get_path().to_path_buf();
        // Note: This is inconsistent, new will assume a directory and filename while load_state assumes nothing
        let initial =
            SnapshotsState::new(base_path.as_path()).expect("Failed to initialize the repository");
        let loaded = SnapshotsState::load(base_path.as_path())
            .expect("Failed to load the repository that was just initialized");
        assert_eq!(initial, loaded);
    }

    #[test]
    fn test_get_set_latest_hash() {
        let known_hash: [u8; 64] = [
            0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11,
            0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF,
            0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11,
            0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF,
            0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF,
        ];
        let latest_hash = Hash::from(known_hash.as_ref());
        let ts = TestSpace::new();
        let mut ts2 = ts.create_child();
        ts2.create_dir("snapshots");
        let working_path = ts.get_path();
        let repository_path = ts2.get_path();
        let known_hash = latest_hash.clone();
        {
            let mut au = AtomicUpdate::new(repository_path, working_path)
                .expect("Failed to initialize atomic updater");
            let mut initial =
                SnapshotsState::new(repository_path).expect("Failed to initialize state");
            initial
                .change_state(&mut au, |state| {
                    state.set_latest_snapshot(Some(latest_hash));
                    Ok(())
                })
                .expect("Failed to change state");
            au.complete().expect("Failed to update files atomically");
        }

        let result = SnapshotsState::load(repository_path).expect("Failed to load state");
        let result_hash = result
            .get_latest_snapshot()
            .expect("Get latest snapshot returned none after a hash was added to latest snapshot");
        assert_eq!(result_hash, known_hash);
    }

    #[test]
    fn test_get_set_working_hash() {
        let known_hash: [u8; 64] = [
            0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11,
            0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF,
            0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11,
            0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF,
            0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF,
        ];
        let latest_hash = Hash::from(known_hash.as_ref());
        let ts = TestSpace::new();
        let mut ts2 = ts.create_child();
        ts2.create_dir("snapshots");
        let working_path = ts.get_path();
        let repository_path = ts2.get_path();
        let known_hash = latest_hash.clone();
        {
            let mut initial =
                SnapshotsState::new(repository_path).expect("Failed to initialize state");
            let mut au = AtomicUpdate::new(repository_path, working_path)
                .expect("Failed to initialize atomic updater");
            initial
                .change_state(&mut au, |state| {
                    state.set_working_snapshot(Some(latest_hash));
                    Ok(())
                })
                .expect("Failed to change state");
            au.complete().expect("Failed to update files atomically");
        }
        let result = SnapshotsState::load(repository_path).expect("Failed to load state");
        let result_hash = result.get_working_snapshot().unwrap();
        assert_eq!(result_hash, known_hash);
    }

    #[test]
    fn test_add_remove_root_snapshots() {
        let known_hash: [u8; 64] = [
            0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11,
            0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF,
            0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11,
            0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF,
            0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF,
        ];
        let latest_hash = Hash::from(known_hash.as_ref());
        let ts = TestSpace::new();
        let mut ts2 = ts.create_child();
        ts2.create_dir("snapshots");
        let working_path = ts.get_path();
        let repository_path = ts2.get_path();
        let known_hash = latest_hash.clone();
        {
            let mut initial =
                SnapshotsState::new(repository_path).expect("Failed to initialize state");
            let mut au = AtomicUpdate::new(repository_path, working_path)
                .expect("Failed to initialize atomic updater");
            initial
                .change_state(&mut au, |state| {
                    state.add_root_node(known_hash.clone());
                    Ok(())
                })
                .expect("Failed to change state");
            au.complete().expect("Failed to complete atomic update");
        }
        {
            let state = SnapshotsState::parse_state_file(
                repository_path
                    .join(SnapshotsState::SNAPSHOTS_PATH)
                    .join(SnapshotsState::STATE_FILE_NAME),
            )
            .expect("Failed to load state");
            assert_eq!(state.root_snapshots.len(), 1);
            assert_eq!(state.root_snapshots[0], latest_hash);
        }
        {
            let mut state = SnapshotsState::parse_state_file(
                repository_path
                    .join(SnapshotsState::SNAPSHOTS_PATH)
                    .join(SnapshotsState::STATE_FILE_NAME),
            )
            .expect("Failed to load state");
            let mut au = AtomicUpdate::load(working_path, repository_path);
            state
                .change_state(&mut au, |state| {
                    state.remove_root_node(&known_hash);
                    Ok(())
                })
                .expect("Failed to change state and remove root snapshot");
            au.complete().expect("Failed to complete atomic update");
            assert_eq!(state.root_snapshots.len(), 0);
        }
        {
            let state = SnapshotsState::parse_state_file(
                repository_path
                    .join(SnapshotsState::SNAPSHOTS_PATH)
                    .join(SnapshotsState::STATE_FILE_NAME),
            )
            .expect("Failed to load state");
            assert_eq!(state.root_snapshots.len(), 0);
        }
    }

    #[test]
    fn test_add_recent_hashes() {
        // Generate 15 random hashes
        let mut test_hashes = Vec::new();
        for _ in 0..15 {
            let hash = Hash::generate_random_hash();
            test_hashes.push(hash);
        }
        let ts = TestSpace::new();
        let ts2 = ts.create_child();
        let working_path = ts.get_path();
        let repository_path = ts2.get_path();
        {
            let mut initial =
                SnapshotsState::new(repository_path).expect("Failed to initialize state");
            let mut au = AtomicUpdate::new(repository_path, working_path)
                .expect("Failed to initialize atomic updater");
            // Add the first 10 hashes to the recent list
            initial
                .change_state(&mut au, |state| {
                    for hash in test_hashes.as_slice()[..10].iter() {
                        state.add_recent_snapshot(hash.clone());
                    }
                    Ok(())
                })
                .expect("Failed to change state");
            au.complete().expect("Failed to complete atomic update");
            assert_eq!(initial.recent_snapshots.len(), 10);
        }
        {
            let mut state = SnapshotsState::parse_state_file(
                repository_path
                    .join(SnapshotsState::SNAPSHOTS_PATH)
                    .join(SnapshotsState::STATE_FILE_NAME),
            )
            .expect("Failed to load state");
            assert_eq!(state.recent_snapshots.len(), 10);
            // Check for those 10 hashes
            for index in 0..state.recent_snapshots.len() {
                assert_eq!(state.recent_snapshots[index], test_hashes[index]);
            }
            let mut au = AtomicUpdate::load(working_path, repository_path);
            state
                .change_state(&mut au, |state_data| {
                    // Add the final 5 hashes
                    for hash in test_hashes.as_slice()[10..].iter() {
                        state_data.add_recent_snapshot(hash.clone());
                    }
                    Ok(())
                })
                .expect("Failed to change state");
            au.complete().expect("Failed to complete atomic update");
        }
        {
            let state = SnapshotsState::parse_state_file(
                repository_path
                    .join(SnapshotsState::SNAPSHOTS_PATH)
                    .join(SnapshotsState::STATE_FILE_NAME),
            )
            .expect("Failed to load state");
            // Check state after removing oldest 5 - so fifth test hash should be first hash in recent
            for index in 5..state.recent_snapshots.len() {
                assert_eq!(state.recent_snapshots[index - 5], test_hashes[index]);
            }
        }
    }

    #[test]
    fn add_remove_end_node_test() {
        let known_hash: [u8; 64] = [
            0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11,
            0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF,
            0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11,
            0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF,
            0xAB, 0x11, 0x0A, 0xFF, 0xAB, 0x11, 0x0A, 0xFF,
        ];
        let latest_hash = Hash::from(known_hash.as_ref());
        let ts = TestSpace::new();
        let mut ts2 = ts.create_child();
        ts2.create_dir("snapshots");
        let working_path = ts.get_path();
        let repository_path = ts2.get_path();
        let known_hash = latest_hash.clone();
        {
            let mut initial =
                SnapshotsState::new(repository_path).expect("Failed to initialize state");
            let mut au = AtomicUpdate::new(repository_path, working_path)
                .expect("Failed to initialize atomic updater");
            initial
                .change_state(&mut au, |state| {
                    state.add_end_node(known_hash.clone());
                    Ok(())
                })
                .expect("Failed to change state");
            au.complete().expect("Failed to complete atomic update");
        }
        {
            let state = SnapshotsState::parse_state_file(
                repository_path
                    .join(SnapshotsState::SNAPSHOTS_PATH)
                    .join(SnapshotsState::STATE_FILE_NAME),
            )
            .expect("Failed to load state");
            assert_eq!(state.end_snapshots.len(), 1);
            assert_eq!(state.end_snapshots[0], latest_hash);
        }
        {
            let mut state = SnapshotsState::parse_state_file(
                repository_path
                    .join(SnapshotsState::SNAPSHOTS_PATH)
                    .join(SnapshotsState::STATE_FILE_NAME),
            )
            .expect("Failed to load state");
            let mut au = AtomicUpdate::load(working_path, repository_path);
            state
                .change_state(&mut au, |state| {
                    state.remove_end_node(&known_hash);
                    Ok(())
                })
                .expect("Failed to change state and remove root snapshot");
            au.complete().expect("Failed to complete atomic update");
            assert_eq!(state.end_snapshots.len(), 0);
        }
        {
            let state = SnapshotsState::parse_state_file(
                repository_path
                    .join(SnapshotsState::SNAPSHOTS_PATH)
                    .join(SnapshotsState::STATE_FILE_NAME),
            )
            .expect("Failed to load state");
            assert_eq!(state.end_snapshots.len(), 0);
        }
    }
}
