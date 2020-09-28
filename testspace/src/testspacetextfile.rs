use crate::{Alphabet, TestSpace, TestSpaceTextFile};
use std::fs;
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::path;

impl TestSpaceTextFile {
    pub fn new(path_to_file: path::PathBuf, alphabet: Alphabet) -> TestSpaceTextFile {
        TestSpaceTextFile {
            alphabet,
            lines_of_text: Vec::new(),
            path_to_file,
            auto_flush: true,
        }
    }

    pub fn get_path(&self) -> &path::Path {
        self.path_to_file.as_path()
    }

    pub fn open_file(path_to_file: path::PathBuf) -> TestSpaceTextFile {
        let file = fs::OpenOptions::new()
            .create(false)
            .read(true)
            .open(path_to_file.as_path())
            .unwrap();
        let reader = BufReader::new(file);
        let mut lines = Vec::new();
        for untested_line in reader.lines() {
            let line = untested_line.unwrap();
            lines.push(line);
        }
        TestSpaceTextFile {
            alphabet: Alphabet::Latin,
            lines_of_text: lines,
            path_to_file,
            auto_flush: true,
        }
    }

    pub fn auto_flush(&mut self, auto_flush: bool) -> &Self {
        self.auto_flush = auto_flush;
        self
    }

    pub fn remove_line(&mut self, line_number: usize) {
        if line_number >= self.lines_of_text.len() {
            panic!(
                "Attempt to remove line {} but there are only {} lines in the file {}",
                line_number,
                self.lines_of_text.len(),
                self.path_to_file.display()
            );
        }
        self.lines_of_text.remove(line_number);
        self.try_flush();
    }

    pub fn insert_line(&mut self, line_number: usize, text: &str) {
        if line_number > self.lines_of_text.len() {
            panic!(
                "Attempt to insert a line at line {} but there are only {} lines in the file {}",
                line_number,
                self.lines_of_text.len(),
                self.path_to_file.display()
            );
        }
        self.lines_of_text.insert(line_number, text.to_string());
        self.try_flush();
    }

    pub fn swap_lines(&mut self, source_number: usize, dest_number: usize) {
        let total_lines = self.lines_of_text.len();
        if source_number >= total_lines || dest_number >= total_lines {
            panic!(
                "Attempt to move line {} to line {}, but there are only {} lines in the file",
                source_number,
                dest_number,
                self.lines_of_text.len()
            );
        }
        self.lines_of_text.swap(source_number, dest_number);
        self.try_flush();
    }

    // Moves a line from the source number to the destination number
    pub fn move_line(&mut self, source_number: usize, dest_number: usize) {
        let total_lines = self.lines_of_text.len();
        if source_number >= total_lines || dest_number >= total_lines {
            panic!(
                "Attempt to move line {} to line {}, but there are only {} lines in the file",
                source_number,
                dest_number,
                self.lines_of_text.len()
            );
        }
        let line_to_move = self.lines_of_text.remove(source_number);
        self.lines_of_text.insert(dest_number, line_to_move);
        self.try_flush();
    }

    // QUESTION: Should we use a zero based index or one based since we are talking about lines
    pub fn change_line(&mut self, line_number: usize, text: &str) {
        if line_number >= self.lines_of_text.len() {
            panic!(
                "Attempt to change line {} but there are only {} lines in the file {}",
                line_number,
                self.lines_of_text.len(),
                self.path_to_file.display()
            );
        }
        self.lines_of_text[line_number] = text.to_string();
        self.try_flush();
    }

    pub fn append_line(&mut self, text: &str) {
        let line_to_insert = text.to_string();
        self.lines_of_text.push(line_to_insert);
        self.try_flush();
    }

    fn try_flush(&mut self) {
        if self.auto_flush {
            self.write_to_disk();
        }
    }

    pub fn write_to_disk(&mut self) {
        // We rewrite the entire file each time, creating it if necessary
        let mut file = fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(self.path_to_file.as_path())
            .unwrap();
        let mut new_line: [u8; 1] = [0];
        '\n'.encode_utf8(&mut new_line);
        // let writer = BufWriter::new(file);
        for line_of_text in self.lines_of_text.iter() {
            file.write(line_of_text.as_bytes()).unwrap();
            file.write(new_line.as_ref()).unwrap();
        }
    }

    pub fn create_copy(&self) -> TestSpaceTextFile {
        let lines_of_text = self.lines_of_text.clone();
        let alphabet = self.alphabet;
        let mut path_to_file = self.path_to_file.clone();
        path_to_file.pop();
        let file_name = TestSpace::get_random_name(15, Alphabet::Latin, Some(".txt"));
        path_to_file.push(file_name);
        let mut copy = TestSpaceTextFile {
            alphabet,
            lines_of_text,
            path_to_file,
            auto_flush: true,
        };
        copy.write_to_disk();
        copy
    }
}
#[cfg(test)]
mod tests {
    use crate::{Alphabet, TestSpace};
    use std::io::BufRead;

    #[test]
    fn create_copy_test() {
        let ts = TestSpace::new();
        let mut txt = ts.create_text_file();
        let test_data = vec!["A", "B", "C", "D"];
        txt.append_line("A");
        txt.append_line("B");
        txt.append_line("C");
        let mut copy = txt.create_copy();
        copy.append_line("D");
        let second_file = copy.get_path();
        let file = std::fs::OpenOptions::new()
            .read(true)
            .open(second_file)
            .unwrap();
        let reader = std::io::BufReader::new(file);
        for (read_line, &test_line) in reader.lines().zip(test_data.iter()) {
            let valid_line = read_line.unwrap();
            assert_eq!(valid_line.as_str(), test_line);
        }
    }

    #[test]
    fn append_line_test() {
        let ts = TestSpace::new();
        let test_data: [&str; 5] = ["a", "b", "c", "d", "e"];
        let mut txt = ts.create_text_file();
        for data in test_data.iter() {
            txt.append_line(data);
        }
        let path = txt.get_path();
        let file = std::fs::OpenOptions::new().read(true).open(path).unwrap();
        let reader = std::io::BufReader::new(file);
        for (read_line, &test_line) in reader.lines().zip(test_data.iter()) {
            let valid_line = read_line.unwrap();
            assert_eq!(valid_line.as_str(), test_line);
        }
    }

    #[test]
    fn remove_line_test() {
        let ts = TestSpace::new();
        let test_data: [&str; 5] = ["a", "b", "c", "d", "e"];
        let mut txt = ts.create_text_file();
        for data in test_data.iter() {
            txt.append_line(data);
        }
        txt.remove_line(3);
        let mut adjusted_data = test_data.to_vec();
        adjusted_data.remove(3);
        let path = txt.get_path();
        let file = std::fs::OpenOptions::new().read(true).open(path).unwrap();
        let reader = std::io::BufReader::new(file);
        for (read_line, &test_line) in reader.lines().zip(adjusted_data.iter()) {
            let valid_line = read_line.unwrap();
            assert_eq!(valid_line.as_str(), test_line);
        }
    }

    #[test]
    fn insert_line_test() {
        let ts = TestSpace::new();
        let test_input: [&str; 4] = ["a", "b", "d", "e"];
        let expected_result: [&str; 5] = ["a", "b", "c", "d", "e"];
        let mut txt = ts.create_text_file();
        for data in test_input.iter() {
            txt.append_line(data);
        }
        txt.insert_line(2, "c");
        let path = txt.get_path();
        let file = std::fs::OpenOptions::new().read(true).open(path).unwrap();
        let reader = std::io::BufReader::new(file);
        for (read_line, &test_line) in reader.lines().zip(expected_result.iter()) {
            let valid_line = read_line.unwrap();
            assert_eq!(valid_line.as_str(), test_line);
        }
    }

    #[test]
    fn insert_line_complex_test() {
        let ts = TestSpace::new();
        let test_input: [&str; 4] = ["a", "b", "c", "d"];
        let expected_result: [&str; 5] = ["a", "c", "b", "a", "d"];
        let mut txt = ts.create_text_file();
        for data in test_input.iter() {
            txt.append_line(data);
        }
        // Move c back one
        txt.move_line(2, 1);
        // Insert an a in front of the d
        txt.insert_line(3, "a");
        let path = txt.get_path();
        let file = std::fs::OpenOptions::new().read(true).open(path).unwrap();
        let reader = std::io::BufReader::new(file);
        for (read_line, &test_line) in reader.lines().zip(expected_result.iter()) {
            let valid_line = read_line.unwrap();
            assert_eq!(valid_line.as_str(), test_line);
        }
    }

    // ["A", "B", "C", "D"]
    // ACBAD

    #[test]
    fn change_line_test() {
        let ts = TestSpace::new();
        let test_data: [&str; 4] = ["z", "b", "c", "d"];
        let test_result: [&str; 4] = ["a", "b", "c", "d"];
        let mut txt = ts.create_text_file();
        for data in test_data.iter() {
            txt.append_line(data);
        }
        txt.change_line(0, "a");
        let path = txt.get_path();
        let file = std::fs::OpenOptions::new().read(true).open(path).unwrap();
        let reader = std::io::BufReader::new(file);
        for (read_line, &test_line) in reader.lines().zip(test_result.iter()) {
            let valid_line = read_line.unwrap();
            assert_eq!(valid_line.as_str(), test_line);
        }
    }

    #[test]
    fn swap_line_test() {
        let ts = TestSpace::new();
        let test_data: [&str; 4] = ["d", "b", "c", "a"];
        let test_result: [&str; 4] = ["a", "b", "c", "d"];
        let mut txt = ts.create_text_file();
        for data in test_data.iter() {
            txt.append_line(data);
        }
        txt.swap_lines(0, 3);
        let path = txt.get_path();
        let file = std::fs::OpenOptions::new().read(true).open(path).unwrap();
        let reader = std::io::BufReader::new(file);
        for (read_line, &test_line) in reader.lines().zip(test_result.iter()) {
            let valid_line = read_line.unwrap();
            assert_eq!(valid_line.as_str(), test_line);
        }
    }

    #[test]
    fn move_line_test() {
        let ts = TestSpace::new();
        let test_data: [&str; 4] = ["d", "a", "b", "c"];
        let test_result: [&str; 4] = ["a", "b", "c", "d"];
        let mut txt = ts.create_text_file();
        for data in test_data.iter() {
            txt.append_line(data);
        }
        // Move line 0 to line 3
        txt.move_line(0, 3);
        let path = txt.get_path();
        let file = std::fs::OpenOptions::new().read(true).open(path).unwrap();
        let reader = std::io::BufReader::new(file);
        for (read_line, &test_line) in reader.lines().zip(test_result.iter()) {
            let valid_line = read_line.unwrap();
            assert_eq!(valid_line.as_str(), test_line);
        }
    }
}
