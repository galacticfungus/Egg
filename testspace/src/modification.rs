use crate::{Alphabet, LineModification, TextModifier};
use std::fmt::Debug;

impl LineModification {
    // Changed(usize, String, String),
    //Insert(usize, String),
    pub fn get_line_number(&self) -> usize {
        match self {
            Self::Insert(line_number, _) => *line_number,
            Self::Changed(line_number, _, _) => *line_number,
            Self::Remove(line_number) => *line_number,
        }
    }
}

impl Debug for LineModification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Insert(line_number, line_inserted) => f.write_fmt(format_args!(
                "Inserted a line at {}, the line was {}",
                line_number + 1,
                line_inserted
            )),
            Self::Changed(line_number, original_line, new_line) => f.write_fmt(format_args!(
                "Changed the line at {}, the original line was '{}', the new line is '{}'",
                line_number + 1,
                original_line,
                new_line
            )),
            Self::Remove(line_number) => {
                f.write_fmt(format_args!("Line {} was removed", line_number + 1))
            }
        }
    }
}

impl TextModifier {
    pub fn new(original_lines: Vec<String>) -> TextModifier {
        TextModifier {
            original: original_lines.clone(),
            modified: original_lines,
            changes: Vec::new(),
            lines_modified: Vec::new(),
        }
    }

    pub fn insert_line(&mut self, line_number: usize, text: String) {
        self.modified.insert(line_number, text.clone());
        self.changes
            .push(LineModification::Insert(line_number, text));
    }

    pub fn change_line(&mut self, line_number: usize, text: String) {
        let mut line_text = text;
        std::mem::swap(&mut self.modified[line_number], &mut line_text);
        self.changes.push(LineModification::Changed(
            line_number,
            self.modified[line_number].clone(),
            line_text,
        ));
    }

    pub fn remove_line(&mut self, line_number: usize) {
        self.modified.remove(line_number);
        self.changes.push(LineModification::Remove(line_number));
    }

    pub fn insert_random_line(&mut self, rng: &mut impl rand::Rng) {
        let line_number = self.get_unique_random_line(rng);
        let actual_line_number = self.get_actual_line_number(line_number);
        let latin = Alphabet::Latin;
        let line_to_insert = latin.get_random_line(rng);
        println!("Adjusted line number is {}", actual_line_number);
        self.modified
            .insert(actual_line_number, line_to_insert.clone());
        self.changes
            .push(LineModification::Insert(line_number, line_to_insert));
    }

    pub fn remove_random_line(&mut self, rng: &mut impl rand::Rng) {
        let line_number = self.get_unique_random_line(rng);
        let actual_line_number = self.get_actual_line_number(line_number);
        println!("Adjusted line number is {}", actual_line_number);
        self.modified.remove(actual_line_number);
        // We store the line number as it would apply to the original document
        self.changes.push(LineModification::Remove(line_number));
    }

    fn get_unique_random_line(&self, rng: &mut impl rand::Rng) -> usize {
        let mut random_number = rng.gen_range(0, self.modified.len());
        let mut attempt: usize = 0;
        while self
            .changes
            .iter()
            .any(|change| change.get_line_number() == random_number)
        {
            random_number = rng.gen_range(0, self.modified.len());
            attempt += 1;
            debug_assert!(attempt < 1000000);
        }
        random_number
    }

    fn get_actual_line_number(&self, line_number: usize) -> usize {
        let adjusted_line = self
            .changes
            .iter()
            .filter(|&change| match change {
                LineModification::Changed(_, _, _) => false,
                LineModification::Remove(line_removed) => {
                    if line_number >= *line_removed {
                        true
                    } else {
                        false
                    }
                }
                LineModification::Insert(line_inserted, _) => {
                    if line_number >= *line_inserted {
                        true
                    } else {
                        false
                    }
                }
            })
            .fold(line_number, |total_offset, change| match change {
                LineModification::Changed(_, _, _) => 0,
                LineModification::Remove(_) => total_offset - 1,
                LineModification::Insert(_, _) => total_offset + 1,
            });
        adjusted_line
    }

    pub fn change_random_line(&mut self, rng: &mut impl rand::Rng) {
        // We always use the line number based on the original file not the one being changed
        // This means we need to adjust the line number by the operations that have already occurred
        // For each removal that occured before this line number we subtract one, for each insertion that occurred before this line number we add one
        let line_number = self.get_unique_random_line(rng);
        let actual_line_number = self.get_actual_line_number(line_number);
        let alpha = Alphabet::Latin;
        let mut new_text = alpha.get_random_line(rng);
        // let final_line = line_number + total_offset;
        println!("Adjusted line number is {}", actual_line_number);
        std::mem::swap(&mut self.modified[actual_line_number], &mut new_text);
        // so new_text is now the original line
        self.changes.push(LineModification::Changed(
            line_number,
            new_text,
            self.modified[actual_line_number].clone(),
        ));
    }

    pub fn get_modified_lines(self) -> (Vec<String>, Vec<LineModification>) {
        // TODO: The history needs to be adjusted here since a remove at line 6 followed by an insert an line 6 looks like a line 6 change to any follow up algorithm
        (self.modified, self.changes)
    }
}

#[cfg(test)]
mod tests {
    use crate::{LineModification, TestSpace, TestSpaceFile, TextModifier};
    use std::collections::HashSet;

    #[test]
    fn test_random_number_uniqueness() {
        let mut rng = rand::thread_rng();
        let mut ts = TestSpace::new();
        let text_file = ts.create_random_text_file(10);
        let mut tsf = TestSpaceFile::from(text_file);
        let original_lines = tsf.read_lines();
        let mut tracker = HashSet::new();
        let mut modifier = TextModifier::new(original_lines);
        for line in 0..10 {
            let random = modifier.get_unique_random_line(&mut rng);
            println!("Random number is {}", random);
            // Add that random number to the numbers that can't be reproduced
            let fake_entry = LineModification::Remove(random);
            modifier.changes.push(fake_entry);
            if tracker.contains(&random) {
                panic!("Non unique random number generated")
            }
            tracker.insert(random);
        }
    }

    #[test]
    fn test_change_random_line() {
        let mut rng = rand::thread_rng();
        let mut ts = TestSpace::new();
        let text_file = ts.create_random_text_file(10);
        let mut tsf = TestSpaceFile::from(text_file);
        let original_lines = tsf.read_lines();
        let mut modifier = TextModifier::new(original_lines.clone());
        modifier.change_random_line(&mut rng);
        let (modified_lines, history) = modifier.get_modified_lines();
        let line_modified = history
            .iter()
            .map(|line| line.get_line_number())
            .next()
            .unwrap();
        for line in 0..original_lines.iter().len() {
            if line == line_modified {
                assert_ne!(original_lines[line], modified_lines[line])
            } else {
                assert_eq!(original_lines[line], modified_lines[line]);
            }
        }
    }

    #[test]
    fn test_remove_random_line() {
        let mut rng = rand::thread_rng();
        let mut ts = TestSpace::new();
        let text_file = ts.create_random_text_file(10);
        let mut tsf = TestSpaceFile::from(text_file);
        let original_lines = tsf.read_lines();
        let mut modifier = TextModifier::new(original_lines.clone());
        modifier.remove_random_line(&mut rng);
        let (modified_lines, history) = modifier.get_modified_lines();
        let line_removed = history
            .iter()
            .map(|line| line.get_line_number())
            .next()
            .unwrap();
        println!("Removed line {}", line_removed);
        assert!(modified_lines.len() < original_lines.len());
        if line_removed < modified_lines.len() {
            assert_ne!(modified_lines[line_removed], original_lines[line_removed]);
        }
    }

    #[test]
    fn test_insert_random_line() {
        let mut rng = rand::thread_rng();
        let mut ts = TestSpace::new();
        let text_file = ts.create_random_text_file(10);
        let mut tsf = TestSpaceFile::from(text_file);
        let original_lines = tsf.read_lines();
        let mut modifier = TextModifier::new(original_lines.clone());
        modifier.insert_random_line(&mut rng);
        let (modified_lines, _) = modifier.get_modified_lines();
        assert!(modified_lines.len() > original_lines.len());
    }

    #[test]
    fn test_adjusted_line_after_remove() {
        let mut rng = rand::thread_rng();
        let mut ts = TestSpace::new();
        let text_file = ts.create_random_text_file(10);
        let mut tsf = TestSpaceFile::from(text_file);
        let original_lines = tsf.read_lines();
        let mut modifier = TextModifier::new(original_lines.clone());
        modifier.remove_random_line(&mut rng);
        // let (modified_lines, history) = modifier.get_modified_lines();
        let line_removed = modifier
            .changes
            .iter()
            .map(|line| line.get_line_number())
            .next()
            .unwrap();
        println!("Removed line {}", line_removed);
        // If the last line wasn't randomly removed we can run the test
        if line_removed != original_lines.len() - 1 {
            let new_line_number = modifier.get_actual_line_number(line_removed + 1);
            assert_eq!(new_line_number, line_removed);
        } else {
            // the last line was randomly removed so we cant test the actual line of a line after that one
        }
    }

    #[test]
    fn test_adjusted_line_after_insert() {
        let mut rng = rand::thread_rng();
        let mut ts = TestSpace::new();
        let text_file = ts.create_random_text_file(10);
        let mut tsf = TestSpaceFile::from(text_file);
        let original_lines = tsf.read_lines();

        let mut modifier = TextModifier::new(original_lines.clone());
        modifier.insert_random_line(&mut rng);
        // let (modified_lines, history) = modifier.get_modified_lines();
        let line_inserted = modifier
            .changes
            .iter()
            .map(|line| line.get_line_number())
            .next()
            .unwrap();
        println!("Inserted line {}", line_inserted);

        let new_line_number = modifier.get_actual_line_number(line_inserted);
        for line in 0..original_lines.len() {
            println!("{} - {}", line, original_lines[line]);
        }
        let (new_lines, _) = modifier.get_modified_lines();
        for line in 0..new_lines.len() {
            println!("{} - {}", line, new_lines[line]);
        }
        assert_eq!(new_line_number, line_inserted + 1);
    }
}
