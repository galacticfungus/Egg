use std::fs;
use std::path;
use std::vec;

#[derive(Clone, Debug)]
pub enum FileItem {
    Directory(path::PathBuf),
    File(path::PathBuf),
}

#[derive(Debug, Clone)]
pub struct FileHistory {
    history: vec::Vec<FileItem>,
    allow_cleanup: bool,
}

impl Default for FileHistory {
    fn default() -> FileHistory {
        FileHistory {
            history: vec::Vec::default(),
            allow_cleanup: true,
        }
    }
}

impl FileHistory {
    pub fn allow_cleanup(&mut self, cleanup: bool) {
        self.allow_cleanup = cleanup;
    }

    pub fn record_directory<P: AsRef<path::Path>>(&mut self, path: P) {
        let path = path.as_ref();
        self.history.push(FileItem::Directory(path.to_path_buf()));
    }

    pub fn record_file<P: AsRef<path::Path>>(&mut self, path: P) {
        let path = path.as_ref();
        self.history.push(FileItem::File(path.to_path_buf()));
    }

    /// Cleans up the testspace, this function can be called when dropping objects, so can't panic
    pub fn cleanup(&mut self) {
        while let Some(file_item) = self.history.pop() {
            match file_item {
                // TODO: Check if the directory or file exists before trying to remove it
                FileItem::Directory(path) => {
                    fs::remove_dir_all(&path).unwrap_or_else(|err| {
                        eprintln!(
                            "Failed to cleanup the test directory {}, error was {}",
                            path.display(),
                            err
                        );
                    });
                    eprintln!("Deleted {}", path.display());
                }
                FileItem::File(path) => {
                    fs::remove_file(path.as_path()).unwrap_or_else(|err| {
                        eprintln!(
                            "Failed to cleanup the test file {}, error was {}",
                            path.display(),
                            err
                        );
                    });
                    eprintln!("Deleted {}", path.display());
                }
            }
        }
        self.history.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::FileHistory;
    use std::fs;

    #[test]
    fn test_history() {
        let mut history = FileHistory::default();
        let mut temp_dir = std::env::temp_dir();
        temp_dir.push("remove_me");
        let temp_path = temp_dir.as_path();
        fs::create_dir(temp_path).unwrap_or_else(|err| {
            panic!(
                "Failed to create the temporary directory, error was {}",
                err
            );
        });
        assert!(temp_path.exists());
        history.record_directory(temp_path);
        history.cleanup();
        assert_eq!(temp_path.exists(), false);
    }

    #[test]
    fn test_file_history() {
        use byteorder::{self, LittleEndian, WriteBytesExt};
        let mut history = FileHistory::default();
        let mut temp_file = std::env::temp_dir();
        temp_file.push("remove_me_123");
        let temp_path = temp_file.as_path();
        {
            let mut file = fs::OpenOptions::new()
                .create(true)
                .create_new(true)
                .write(true)
                .read(true)
                .open(temp_path)
                .unwrap_or_else(|err| {
                    panic!("Failed to create the temp file, error was {}", err);
                });
            file.write_u64::<LittleEndian>(12345).unwrap_or_else(|err| {
                panic!(
                    "Failed writing test data during history test, error was {}",
                    err
                );
            });
        }
        assert!(temp_path.exists());
        history.record_file(temp_path);
        history.cleanup();
        assert_eq!(temp_path.exists(), false);
    }
}
