use super::history;
use crate::testspace;
use crate::Alphabet;
use crate::TestSpace;
use crate::TestSpaceFile;
use rand::{self, Rng};
use std::fs;
use std::io::{self, Seek, SeekFrom, Write};
use std::path;

impl TestSpaceFile {
    pub fn new(test_space: &TestSpace) -> TestSpaceFile {
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
    pub fn with_suffix(test_space: &TestSpace, suffix: &str) -> TestSpaceFile {
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

    pub fn with_name<F: AsRef<path::Path>>(file_name: F, test_space: &TestSpace) -> TestSpaceFile {
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
    /// Creates a randomly named file in the given directory, returns the path
    fn create_rand_file(base_path: &path::Path, suffix: Option<&str>) -> path::PathBuf {
        let mut random_name: String = TestSpace::get_random_string(9);
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

    pub fn open_file(&self) -> fs::File {
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

    // Returns a vector of strings that represent each line that was read from the tsf file
    pub fn read_lines(&mut self) -> Vec<String> {
        use io::BufRead;
        let mut temp_file = self.get_file_object();
        temp_file.seek(SeekFrom::Start(0)).unwrap_or_else(|err| {
            panic!(
                "Failed to seek to start of file while reading all lines, error was {}",
                err
            )
        });
        let reader = io::BufReader::new(temp_file);
        let lines = reader.lines().map(|line| line.unwrap()).collect();
        lines
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
        temp_file
            .write(random_bytes.as_slice())
            .unwrap_or_else(|error| {
                panic!("Failed to write random bytes to a TSF, error was {}", error)
            });
    }

    pub fn write_random_text(&mut self, alphabet: Alphabet, lines_to_write: usize) {
        let mut rng = rand::thread_rng();
        let characters_to_use = alphabet.get_range();
        let mut text_to_write = String::new();
        for line in 0..lines_to_write {
            let number_of_words = rng.gen_range(5, 13);
            let mut line_to_write = String::new();
            for word_count in 0..number_of_words {
                if word_count != 0 {
                    line_to_write.push(' ');
                }
                let letter_count: usize = rng.gen_range(2, 7);
                let word: String = rng
                    .sample_iter(&characters_to_use)
                    .map(|index| unsafe { std::char::from_u32_unchecked(index) })
                    .filter(|&character| character != '\n' || character != '\r')
                    .take(letter_count)
                    .collect();
                line_to_write.push_str(word.as_str());
            }
            line_to_write.push('\n');
            // Add the line to the text
            text_to_write.push_str(line_to_write.as_str());
        }
        let mut file = self.get_file_object();
        file.write(text_to_write.as_bytes())
            .unwrap_or_else(|error| {
                panic!("Failed to write random text to a TSF, error was {}", error)
            });
    }

    pub fn write_lines(&mut self, lines_to_write: &[String]) {
        let mut file_object = self.get_file_object();
        for line in lines_to_write {
            file_object.write_fmt(format_args!("{}\n", line)).unwrap();
        }
    }

    pub fn write_bytes(&self, bytes_to_write: &[u8]) {
        let mut file_object = self.get_file_object();
        // TODO: We can now return a result in a test
        file_object.write_all(bytes_to_write).unwrap();
    }

    pub fn get_path(&self) -> &path::Path {
        self.path_to_file.as_path()
    }
}

#[cfg(test)]
mod tests {
    use crate::TestSpace;
    use crate::TestSpaceFile;

    #[test]
    fn modify_text_file_text() {
        let mut ts = TestSpace::new();
        let text_file = ts.create_random_text_file(30);
        let tsf = TestSpaceFile::from(text_file.as_path());
        // tsf.remove_line();
        // tsf.insert_line();
        // tsf.change_line();
    }

    #[test]
    fn test_write_random_bytes() {
        let ts = TestSpace::new();
        {
            let mut tsf = ts.create_tsf();
            tsf.write_random_bytes(1024);
            let path_to_file = tsf.get_path();
            assert!(path_to_file.exists());
            let data = path_to_file.metadata().unwrap_or_else(|error| {
                panic!(
                    "Failed to obtain the metadata of a randomly created file, error was {}",
                    error
                );
            });
            let file_size = data.len();
            // Check that we have written 1024 bytes
            assert_eq!(file_size, 1024);
        }
    }
    #[test]
    fn test_write_random_text() {
        use std::fs;
        use std::io::{self, BufRead};
        let ts = TestSpace::new().allow_cleanup(false);
        let mut tsf = ts.create_tsf();
        tsf.write_random_text(crate::Alphabet::Latin, 10);
        let path_to_file = tsf.get_path();
        // Open the tsf file
        let mut file = fs::OpenOptions::new()
            .read(true)
            .open(path_to_file)
            .unwrap();
        let reader = io::BufReader::new(file);
        // Count the number of lines
        let line_count = reader.lines().count();
        assert_eq!(line_count, 10);
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
