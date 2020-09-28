use std::path;

mod alphabet;
mod history;
mod modification;
mod testspace;
mod testspacefile;
mod testspacetextfile;

pub struct TestSpace {
    working_directory: path::PathBuf,
    history: history::FileHistory,
    allow_cleanup: bool,
}

pub struct TestSpaceFile {
    path_to_file: path::PathBuf,
    history: history::FileHistory,
    allow_cleanup: bool,
}

#[derive(Copy, Clone)]
pub enum Alphabet {
    Arabic,
    Chinese,
    Cyrillic,
    Latin,
}
pub enum LineModification {
    Insert(usize, String),
    Remove(usize),
    Changed(usize, String, String),
}

pub struct TestSpaceTextFile {
    path_to_file: path::PathBuf,
    alphabet: Alphabet,
    lines_of_text: Vec<String>,
    auto_flush: bool, // After each edit do we automatically write the changes to disk
}

pub struct TextModifier {
    original: Vec<String>,
    modified: Vec<String>,
    changes: Vec<LineModification>,
    lines_modified: Vec<usize>,
}
