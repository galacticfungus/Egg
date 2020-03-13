
use super::history;
use crate::testspace;
use rand::{self, Rng};
use std::fs;
use std::io::{self, Seek, SeekFrom, Write};
use std::path;

pub struct TestSpaceFile {
    path_to_file: path::PathBuf,
    history: history::FileHistory,
    allow_cleanup: bool,
}

impl TestSpaceFile {
    pub fn new(test_space: &testspace::TestSpace) -> TestSpaceFile {
        let path = TestSpaceFile::create_rand_file(test_space.get_path(), None);
        let mut history = history::FileHistory::default();
        history.record_file(path.as_path());
        TestSpaceFile {
            path_to_file: path,
            history,
            allow_cleanup: test_space.is_cleaning(),
        }
    }

    /// Create a new randomly named test space file but with a specific suffix attached to the file name
    pub fn with_suffix(test_space: &testspace::TestSpace, suffix: &str) -> TestSpaceFile {
        let path = TestSpaceFile::create_rand_file(test_space.get_path(), Some(suffix));
        let mut history = history::FileHistory::default();
        history.record_file(path.as_path());
        TestSpaceFile {
            path_to_file: path,
            history,
            allow_cleanup: test_space.is_cleaning(),
        }
    }

    /// Convert any type that can be converted to a &Path to a TestSpaceFile
    fn from_path<P: AsRef<path::Path>>(path: P) -> TestSpaceFile {
        let path = path.as_ref();
        let mut history = history::FileHistory::default();
        history.record_file(path);
        TestSpaceFile {
            path_to_file: path.to_path_buf(),
            history,
            allow_cleanup: false,
        }
    }

    pub fn with_name<F: AsRef<path::Path>>(file_name: F, test_space: &testspace::TestSpace) -> TestSpaceFile {
        let path = file_name.as_ref();
        let mut history = history::FileHistory::default();
        let new_path = test_space.get_path().join(path);
        history.record_file(new_path.as_path());
        TestSpaceFile {
            path_to_file: new_path,
            history,
            allow_cleanup: test_space.is_cleaning(),
        }
    }
}

impl From<&path::Path> for TestSpaceFile {
    fn from(path: &path::Path) -> Self {
        TestSpaceFile::from_path(path)
    }
}

impl From<path::PathBuf> for TestSpaceFile {
    fn from(path: path::PathBuf) -> Self {
        TestSpaceFile::from_path(path)
    }
}

impl Drop for TestSpaceFile {
    fn drop(&mut self) {
        if self.allow_cleanup {
            self.history.cleanup()
        }
    }
}

impl TestSpaceFile {
    /// Creates a randomly named file in the given directory, returns both the path and the File object
    fn create_rand_file(base_path: &path::Path, suffix: Option<&str>) -> path::PathBuf {
        let mut random_name: String = testspace::TestSpace::get_random_string(9);
        if let Some(suffix) = suffix {
            random_name.push_str(suffix);
        }
        let file_path = base_path.join(random_name);
        file_path
    }
}

impl TestSpaceFile {
    /// Should the TestSpace clean up after itself
    pub fn allow_cleanup(mut self, allow_cleanup: bool) -> Self {
        self.allow_cleanup = allow_cleanup;
        self
    }

    pub fn open_file(& self) -> fs::File {
        self.get_file_object()
    }

    fn get_file_object(&self) -> fs::File {
        println!("path to file is {}", self.path_to_file.display());
        fs::OpenOptions::new()
          .read(true)
          .write(true)
          .create(true)
          .open(self.path_to_file.as_path())
          .unwrap_or_else(|err| {
              panic!(
                  "Failed to create a TestSpaceFile from a path, error was {}, path was {}",
                  err,
                  self.path_to_file.display()
              );
          })
    }

    /// Append a line to the TestSpaceFile
    pub fn append_line(&mut self, line_to_append: &str) {
        use std::io::{self, Write};
        let mut temp_file = self.get_file_object();
        // position buffer to end of file
        temp_file.seek(io::SeekFrom::End(0)).unwrap_or_else(|err| {
            panic!(
                "TestSpaceFile: Append line failed while seeking to end of file: {}",
                err
            );
        });
        temp_file
          .write(line_to_append.as_bytes())
          .unwrap_or_else(|err| {
              panic!(
                  "TestSpaceFile: Append line in the text line failed with the error: {}",
                  err
              );
          });

        temp_file.write("\n".as_bytes()).unwrap_or_else(|err| {
            panic!(
                "TestSpaceFile: Failed to write new line character in append line, error was {}",
                err
            );
        });
        temp_file
          .seek(io::SeekFrom::Start(0))
          .unwrap_or_else(|err| {
              panic!(
                  "TestSpaceFile: Append line failed while resetting seek position, error was {}",
                  err
              );
          });
    }

    pub fn read_line(&mut self, line_number: usize) -> String {
        use std::error::Error;
        use std::io::SeekFrom;
        use std::io::{self, BufRead};
        let mut temp_file = self.get_file_object();
        temp_file.seek(SeekFrom::Start(0)).unwrap_or_else(|err| {
            panic!("Could not seek to the start of the file, Error was {}", err)
        });
        let reader = io::BufReader::new(temp_file);
        let line = reader.lines().nth(line_number).unwrap_or_else(|| {
            panic!("Tried to read line {} but it returned None", line_number);
        });

        line.unwrap_or_else(|err| {
            panic!(
                "Failed to read a line, requested line was {}\nError was: {}\n",
                line_number,
                err.description()
            );
        })
    }

    pub fn read_u16(&mut self) -> u16 {
        use byteorder::{self, ReadBytesExt};
        let mut temp_file = self.get_file_object();
        let data = temp_file
            .read_u16::<byteorder::LittleEndian>()
            .unwrap_or_else(|err| {
                panic!("Failed to read u16, error was {}", err);
            });
        data
    }

    pub fn set_position(&mut self, pos: u64) {
        let mut temp_file = self.get_file_object();
        temp_file.seek(SeekFrom::Start(pos)).unwrap_or_else(|err| {
            panic!(
                "Failed seeking to the file position {}, error was {}",
                pos, err
            );
        });
    }

//    pub fn get_position(&mut self) -> u64 {
//        // Wont work now
//        let mut temp_file = self.get_file_object();
//        temp_file.seek(SeekFrom::Current(0)).unwrap_or_else(|err| {
//            panic!("Failed to get the current file position, error was {}", err);
//        })
//    }

    pub fn write_random_bytes(&mut self, amount: usize) {
        let mut rng = rand::thread_rng();
        let random_bytes: Vec<u8> = rng
            .sample_iter(&rand::distributions::Standard)
            .take(amount)
            .collect();
        let mut temp_file = self.get_file_object();
        temp_file.write(random_bytes.as_slice()).unwrap_or_else(|error| {
            panic!("Failed to write random bytes to a TSF, error was {}", error)
        });
    }

    pub fn get_path(&self) -> &path::Path {
        self.path_to_file.as_path()
    }

    //    fn apply_to_3<F>(f: F) -> i32 where
    // The closure takes an `i32` and returns an `i32`.
    //      F: Fn(i32) -> i32

//    pub fn close_file_while<F>(&mut self, f: F)
//    where
//        F: FnOnce(&path::Path),
//    {
//        //First close the file
//        self.temp_file = None;
//        // Call user code
//        f(self.path_to_file.as_path());
//        //Reopen the file
//        self.temp_file = Some(
//            fs::OpenOptions::new()
//                .read(true)
//                .write(true)
//                .create(true)
//                .open(self.path_to_file.as_path())
//                .unwrap_or_else(|err| {
//                    panic!(
//                        "Failed to reopen a TestSpaceFile after closing it, error was {}",
//                        err
//                    );
//                }),
//        );
//    }
}

#[cfg(test)]
mod tests {
    use crate::testspace::TestSpace;
    use crate::TestSpaceFile;
    use std::env;
    use std::io::Read;
    use byteorder::ReadBytesExt;

    #[test]
    fn test_write_random_bytes() {
        let ts = TestSpace::new();
        {
            let mut tsf = ts.create_tsf();
            tsf.write_random_bytes(1024);
            let path_to_file = tsf.get_path();
            assert!(path_to_file.exists());
            let data = path_to_file.metadata().unwrap_or_else(|error| {
                panic!("Failed to obtain the metadata of a randomly created file, error was {}", error);
            });
            let file_size = data.len();
            // Check that we have written 1024 bytes
            assert_eq!(file_size, 1024);
        }
    }

    #[test]
    fn append_line() {
        let ts = TestSpace::new();
        let mut tfs = ts.create_tsf();
        tfs.append_line("a line");
        let mut file = tfs.get_file_object();
        let mut buffer = String::new();
        file.read_to_string(&mut buffer).unwrap_or_else(|err| {
            panic!("Failed to read the appended line, error was {}", err);
        });
        assert_eq!(buffer, "a line\n");
    }

    #[test]
    fn read_line() {
        let ts = TestSpace::new();
        let mut tfs = ts.create_tsf();
        tfs.append_line("read me");
        let line_read = tfs.read_line(0);
        assert_eq!(line_read, "read me");
        drop(tfs);
        drop(ts);
    }

    #[test]
    fn read_middle_line() {
        let ts = TestSpace::new();
        let mut tfs = ts.create_tsf();
        tfs.append_line("first line");
        tfs.append_line("middle line");
        tfs.append_line("last line");
        let middle_line = tfs.read_line(1);
        assert_eq!(middle_line, "middle line");
    }

    #[test]
    fn create_subspace_file() {
        let ts = TestSpace::new();
        let mut tsf = ts.create_tsf();
        // Files are only created when used so we get a file object then drop it after the test
        let file = tsf.get_file_object();
        assert!(tsf.path_to_file.exists());
        drop(file)
    }
}
