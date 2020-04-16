use crate::history::{self};
use std::path;
use std::fs;
use rand::{self, Rng};
use crate::testspacefile::TestSpaceFile;

pub struct TestSpace {
  working_directory: path::PathBuf,
  history: history::FileHistory,
  allow_cleanup: bool,
}

impl TestSpace {
  const SUFFIX: &'static str = "_ts";
  /// Creates a test space that represents a directory in the temporary system directory.
  pub fn new() -> TestSpace {
    let temp_path = std::env::temp_dir();
    let random_path = TestSpace::create_rand_dir(temp_path.as_path());
    let mut history = history::FileHistory::default();
    history.record_directory(random_path.as_path());
    TestSpace {
      working_directory: random_path,
      history,
      allow_cleanup: true
    }
  }

  pub(crate) fn get_random_string(length: usize) -> String {
    rand::thread_rng()
      .sample_iter(&rand::distributions::Alphanumeric)
      .take(length)
      .collect()
  }

  /// Creates a random directory in the systems temporary directory, all temp space directories are
  /// prefaced with test_space
  fn create_rand_dir(base_path: &path::Path) -> path::PathBuf {
    let suffix: String = TestSpace::get_random_string(15);
    let temp_name = suffix + TestSpace::SUFFIX;
    let mut path_to_dir = base_path.join(temp_name);
    while path_to_dir.exists() {
      let suffix: String = TestSpace::get_random_string(15);
      let temp_name = suffix + TestSpace::SUFFIX;
      path_to_dir = base_path.join(temp_name);
    }
    // Create the randomly named directory
    fs::create_dir(path_to_dir.as_path()).unwrap_or_else(|err| {
      panic!(
        "Creating the random directory {} failed. The system reported the following error {}",
        path_to_dir.display(),
        err
      );
    });
    // History must be recorded in the function calling this one, 
    // FIXME: Probably should record the directory being created in the function that does the actual creating
    // IDEA: An idea
    // BUG: A bug
    // TODO: A todo
    path_to_dir
  }
}

impl TestSpace {
  /// Get the actual directory that this test space is operating in
  pub fn get_path(&self) -> &path::Path {
    self.working_directory.as_path()
  }

  pub fn is_cleaning(&self) -> bool {
    self.allow_cleanup
  }

  /// Should the TestSpace clean up after itself
  pub fn allow_cleanup(mut self, allow_cleanup: bool) -> Self {
    self.allow_cleanup = allow_cleanup;
    self
  }

  pub fn create_random_files(&mut self, amount_to_create: u8, file_size: usize) -> Vec<path::PathBuf> {
    let mut path_list = Vec::new();
    for _ in 0..amount_to_create {
      let mut tsf = TestSpaceFile::with_suffix(self, ".file").allow_cleanup(false);
      tsf.write_random_bytes(file_size);
      path_list.push(tsf.get_path().to_path_buf());
      // We let the testspace do all of the cleanup in this case
      self.history.record_file(tsf.get_path());
    }
    path_list
  }

  pub fn create_file<P: AsRef<path::Path>>(&mut self, file_to_create: P, file_size: usize) -> TestSpaceFile {
    let mut path_list = Vec::new();
    // let target_path = self.working_directory.join(file_name.as_ref());
    let mut tsf = TestSpaceFile::from(file_to_create.as_ref());
    tsf.write_random_bytes(file_size);
    path_list.push(tsf.get_path().to_path_buf());
    // We let the testspace do all of the cleanup in this case
    self.history.record_file(tsf.get_path());
    tsf
  }

  /// Creates a number of random files bThe directory to create the files in must exist
  pub fn create_files<S: AsRef<str>>(&mut self, path_to_directory: S, file_list: &[&str], file_size: usize) -> Vec<TestSpaceFile> {
    let mut test_files = Vec::new();
    let mut path_list = Vec::new();
    // let file_folder = self.create_dir(path_to_directory.as_ref());
    let path_to_files = self.working_directory.join(path_to_directory.as_ref());
    for file in file_list {
      let mut tsf = TestSpaceFile::from(path_to_files.as_path().join(file));
      tsf.write_random_bytes(file_size);
      path_list.push(tsf.get_path().to_path_buf());
      self.history.record_file(tsf.get_path());
      test_files.push(tsf);
    }
    test_files
  }

  /// Creates a test space inside the current test space
  pub fn create_child(&self) -> TestSpace {
    let new_space = TestSpace::create_rand_dir(self.working_directory.as_path());
    let mut history = history::FileHistory::default();
    history.record_directory(new_space.as_path());
    TestSpace {
      working_directory: new_space,
      history,
      allow_cleanup: self.allow_cleanup
    }
  }

  /// Creates a directory in this test spaces directory
  pub fn create_test_path(&mut self) -> path::PathBuf {
    let new_path = TestSpace::create_rand_dir(self.working_directory.as_path());
    self.history.record_directory(new_path.as_path());
    new_path
  }

  /// Creates a directory in the test space with the specified name
  pub fn create_dir<P: AsRef<str>>(&mut self, folder_name: P) -> path::PathBuf {
    let folder_name = folder_name.as_ref();
    let folder_path = self.get_path().join(folder_name);
    fs::create_dir(folder_path.as_path()).unwrap_or_else(|err| {
      panic!("Failed to create a named folder, path to TestSpace was {}, folder being created was {}, the error was {}", self.working_directory.display(), folder_path.display(), err);
    });
    self.history.record_directory(folder_path.as_path());
    folder_path
  }

  /// Creates a randomly named file and returns a TestSpaceFile to act on that file
  pub fn create_tsf(&self) -> crate::testspacefile::TestSpaceFile {
    crate::testspacefile::TestSpaceFile::new(self)
  }
}

impl Drop for TestSpace {
  fn drop(&mut self) {
    if self.allow_cleanup {
      self.history.cleanup();
    }
  }
}
#[cfg(test)]
mod tests {
  use crate::testspace::TestSpace;
  use std::env;
  use std::io::Read;

    #[test]
    fn create_test_path() {
        let mut ts = TestSpace::new().allow_cleanup(true);
        let mut dir_path = ts.create_test_path();
        let nested_path = dir_path.to_path_buf();
        dir_path.pop();
        assert_eq!(dir_path.as_path(), ts.get_path());
        assert!(nested_path.exists());
        drop(ts);
        assert_eq!(nested_path.exists(), false);
        assert_eq!(dir_path.exists(), false);
    }

    #[test]
    fn create_subspace() {
        let ts = TestSpace::new();
        let root_path = ts.get_path();
        let sub = ts.create_child();
        let sub_path = sub.get_path();
        assert_ne!(root_path, sub_path);
        let mut adjusted_sub_path = sub_path.to_path_buf();
        adjusted_sub_path.pop();
        assert_eq!(root_path, adjusted_sub_path.as_path());
    }

    #[test]
    fn new_and_get_path() {
        let test_space = TestSpace::new();
        let current_path = test_space.get_path();
        assert_eq!(current_path.exists(), true);
        drop(test_space);
    }

    #[test]
    fn create_random_directory() {
        use std::fs;
        let temp_folder = env::temp_dir();
        let random_path = TestSpace::create_rand_dir(temp_folder.as_path());
        assert_eq!(random_path.exists(), true);
        fs::remove_dir(random_path).unwrap_or_else(|err| {
        panic!("Could not delete test directory after test, error was {}", err);
        });
    }

    #[test]
    fn get_random_string() {
        let random_name = TestSpace::get_random_string(15);
        assert_eq!(random_name.len(), 15);
    }

    #[test]
    fn create_random_files() {
        let file_list;
        {
        let mut ts = TestSpace::new();
        file_list = ts.create_random_files(5, 2048);
        println!("{:?}", file_list.as_slice());
        assert_eq!(file_list.len(), 5);
        for file in &file_list {
            assert!(file.exists());
        }
        }
        // Check that the files no longer exist when the TestSpace is out of scope
        for file in file_list {
        assert_eq!(file.exists(), false);
        }
    }

    #[test]
    fn test_create_random_files() {
        let mut ts = TestSpace::new();
        let file_list = ts.create_random_files(3, 2048);
        assert!(file_list.iter().all(|path| path.exists()));
        assert!(file_list.iter().all(|path| path.to_str().unwrap().ends_with(".file")));
    }

    #[test]
    fn test_create_file_list() {
        let mut ts = TestSpace::new();
        let file_list = vec!("a", "b", "c");
        ts.create_dir("cc");
        let created_files = ts.create_files("cc", file_list.as_slice(), 1024);
        let cc_path = ts.get_path().join("cc");
        let a_path = cc_path.join("a");
        let b_path = cc_path.join("b");
        let c_path = cc_path.join("c");
        assert!(a_path.exists());
        assert!(b_path.exists());
        assert!(c_path.exists());
    }
}