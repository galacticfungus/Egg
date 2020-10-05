use crate::error::{Error, UnderlyingError};
use crate::hash::Hash;
use crate::snapshots::{FileMetadata, Snapshot};
use crate::storage::stream::{ReadEggExt, WriteEggExt};
use ahash;
use byteorder::LittleEndian;
use byteorder::{ReadBytesExt, WriteBytesExt};
use smallvec::SmallVec;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::fs;
use std::io::{self, BufRead};
use std::path;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

type Result<T> = std::result::Result<T, Error>;

// TODO: Redo this as a Vec and HashSet

/// Represents a file in the working directory that we may wish to snapshot or otherwise investigate,
/// This structure does not contain the path of the file since the path is used as a key inside a map of WorkingFiles
struct WorkingFile {
    hash: Option<Hash>,
    file_size: u64,
    modified_time: u128,
}

// TODO: String can't be used here, we must use a byte array since the data may not be valid utf8
#[derive(PartialEq)]
enum ProspectiveDifference<'a> {
    DuplicateRemove(&'a str, VecDeque<usize>),
    DuplicateInsert(&'a str, VecDeque<usize>),
    Remove(&'a str, usize),
    Insert(&'a str, usize),
}

impl<'a> std::fmt::Debug for ProspectiveDifference<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // Convert to BTreeSet
            ProspectiveDifference::DuplicateInsert(line, duplicates) => f.write_fmt(format_args!(
                "The line '{}' has multiple inserts at {:?} in the new file",
                line, duplicates
            )),
            ProspectiveDifference::DuplicateRemove(line, duplicates) => f.write_fmt(format_args!(
                "The line '{}' has duplicate removals at {:?} in the original file",
                line, duplicates
            )),
            ProspectiveDifference::Insert(line, line_number) => f.write_fmt(format_args!(
                "The line {} was inserted at line {} in the new file",
                line, line_number
            )),
            ProspectiveDifference::Remove(line, line_number) => f.write_fmt(format_args!(
                "The line {} was removed from the original file at line {}",
                line, line_number
            )),
        }
    }
}

pub enum ProspectiveMove<'a> {
    // First usize is the slice line, second usize is the previous line
    UnknownMove(usize, usize, &'a str),
    // First usize is original line, second usize is the edited line
    Move(usize, usize, &'a str),
    // Multiple line move - original line, edit line, number of lines, slice of lines
    MultipleLines(usize, usize, usize, &'a [&'a str]),
}

impl<'a> std::fmt::Display for ProspectiveMove<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProspectiveMove::UnknownMove(slice_line, previous_line, line_text) => f.write_fmt(format_args!("The line '{}' was moved from slice line {} to previous line {}", line_text, slice_line, previous_line)),
            ProspectiveMove::Move(original_line, previous_line, line_data) => f.write_fmt(format_args!("The line '{}' was moved from line {} in the original file to line {} in the new file", line_data, original_line, previous_line)),
            ProspectiveMove::MultipleLines(original_line, previous_line, line_count, lines) => {
                f.write_fmt(format_args!("The lines {} to {} were moved to line {}", original_line, original_line + line_count, previous_line))
            },
        }
    }
}

// Prospective moves are guarenteed to be moves however the lines that are being moved may be changed
// TODO: This all needs to be changed to a
struct Move<'a> {
    line: &'a str,
    source_line: usize,
    new_line: usize,
}

impl<'a> Move<'a> {
    pub fn get_lines(&self) -> (usize, usize) {
        (self.source_line, self.new_line)
    }
}

impl<'a> std::fmt::Debug for Move<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "The line '{}' moved from {} to {}",
            self.line, self.source_line, self.new_line
        ))
    }
}

// Represents a diff where the underlying data is not owned
pub enum Diff<'a> {
    Insert(&'a str, usize),
    Remove(&'a str, usize),
    DuplicateRemoves(&'a str, SmallVec<[usize; 5]>),
    DuplicateInserts(&'a str, SmallVec<[usize; 5]>),
    Moved(&'a str, usize, usize),
    Changed(&'a str, String, usize),
}
#[derive(PartialEq, Hash, Eq, Debug)]
pub struct LineMoved {
    source_line: usize, // Lines are always moved from the original document to the edited document
    destination_line: usize, 
}

impl std::fmt::Display for LineMoved {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{} to {}", self.source_line, self.destination_line))
    }
}

pub struct RawZip<A, B> {
    a: A,
    b: B,
}

impl<A, B> Iterator for RawZip<A, B>
where
    A: Iterator,
    B: Iterator,
{
    type Item = (Option<A::Item>, Option<B::Item>);

    fn next(&mut self) -> Option<Self::Item> {
        match (self.a.next(), self.b.next()) {
            (None, None) => None,
            (a, b) => Some((a, b)),
        }
        // let x = self.a.next();
        // let y = self.b.next();
        // if x.is_none() && y.is_none() {
        //     return None;
        // }
        // // As long as one of the iterators is returning Some we continue returning results
        // Some((x,y))
    }
}

impl<A: Iterator, B: Iterator> RawZip<A, B> {
    pub fn new(a: A, b: B) -> RawZip<A, B> {
        RawZip { a, b }
    }
}

impl WorkingFile {
    pub fn is_hashed(&self) -> bool {
        self.hash.is_some()
    }
    pub fn hash(&self) -> Option<&Hash> {
        self.hash.as_ref()
    }

    pub fn filesize(&self) -> u64 {
        self.file_size
    }

    pub fn modified_time(&self) -> u128 {
        self.modified_time
    }
}

/// Contains a number of helpful functions for dealing with the Repositories working directory
/// Primarily it provides an interface to check if a file(s) have changed since a snapshot was taken
pub struct WorkingDirectory<'a> {
    working_files: HashMap<path::PathBuf, WorkingFile>,
    path_to_working: &'a path::Path,
}

impl<'a> WorkingDirectory<'a> {
    pub fn new(path_to_working: &'a path::Path) -> WorkingDirectory {
        WorkingDirectory {
            working_files: HashMap::new(),
            path_to_working,
        }
    }

    /// Looks at all the files in a directory and stores file size, file name and last modified file time,
    fn index_directory(
        path_to_search: &path::Path,
        files_found: &mut HashMap<path::PathBuf, WorkingFile>,
        directories_found: &mut Vec<PathBuf>,
    ) -> Result<()> {
        let items_found = match fs::read_dir(path_to_search) {
            Ok(result) => result,
            Err(error) => unimplemented!(),
        };
        for item in items_found {
            let valid_item = match item {
                Ok(valid_item) => valid_item,
                Err(error) => unimplemented!(),
            };
            let path = valid_item.path();

            let file_type = match valid_item.file_type() {
                Ok(item_type) => item_type,
                Err(error) => unimplemented!(),
            };
            if file_type.is_dir() {
                // TODO: check if the path is .egg as we need to ignore this
                // TODO: Technically we need to filter out the repository directory, so the .egg folder inside the working directory
                // TODO: Check for infinite recursion
                // self.path_to_working.join(".egg");
                directories_found.push(path);
            } else {
                // Add the file to the list
                let metadata = match valid_item.metadata() {
                    Ok(metadata) => metadata,
                    Err(error) => unimplemented!(),
                };
                let modified_time = WorkingDirectory::get_modified_time(&metadata);
                // let time_modified = date.elapsed();
                let data = WorkingFile {
                    hash: None, // We only need to provide a hash if the file system can't provide a length or modification time for the given file
                    file_size: metadata.len(),
                    modified_time,
                };
                files_found.insert(path, data);
            }
        }
        Ok(())
    }

    // Retrieves the last modified time of a file with microsecond resolution
    fn get_modified_time(file_metadata: &fs::Metadata) -> u128 {
        match file_metadata.modified() {
            Ok(valid_date) => match valid_date.duration_since(SystemTime::UNIX_EPOCH) {
                Ok(time_modified) => time_modified.as_micros(),
                Err(error) => unimplemented!(), // This means that the file was modified before the UNIX EPOCH
            },
            Err(error) => unimplemented!(), // File system does not support obtaining the last modified file time - TODO: Fallback to using a hash and display a warning
        }
    }

    fn index_repository(&self) -> Result<HashMap<path::PathBuf, WorkingFile>> {
        // All files must be relative to the working directory as that is how snapshot paths are stored
        let mut directories_to_search = Vec::new();
        let mut files_found = HashMap::new();
        WorkingDirectory::index_directory(
            self.path_to_working,
            &mut files_found,
            &mut directories_to_search,
        )?;
        // FIXME: This is subject to infinite recursion if user has created links to parent directory, the fix will need to be in index_directory
        while let Some(path_to_search) = directories_to_search.pop() {
            WorkingDirectory::index_directory(
                path_to_search.as_path(),
                &mut files_found,
                &mut directories_to_search,
            )?;
        }
        Ok(files_found)
    }

    /// Compares the list of hashed paths with the working directory
    pub fn get_changed_files<'b>(&self, stored_files: &'b [FileMetadata]) -> Vec<&'b path::Path> {
        // Get snapshot paths and their hashes
        // Lookup path in changed files and compare hashes
        // Get an update to date list of all files in the repositories working directory
        // let root_dir = ;
        let mut changed_files = Vec::new();
        let files_found = match self.index_repository() {
            Ok(files_found) => files_found,
            Err(error) => unimplemented!(),
        };
        for stored_file in stored_files {
            let working_file = match files_found.get(stored_file.path()) {
                Some(working_file) => working_file,
                None => {
                    // The working directory has no file with that path
                    // TODO: Every changed file needs a status associated with it - ie sometimes a file is deleted renamed or created, as opposed to just edited
                    changed_files.push(stored_file.path()); // File exists in list but not in repository
                    break;
                }
            };
            if working_file.file_size != stored_file.filesize() {
                // File sizes do not match
                changed_files.push(stored_file.path());
            }
            if working_file.modified_time != stored_file.modified_time() {
                // Time of modification does not match
                changed_files.push(stored_file.path());
            }
        }
        changed_files
    }

    /// Compares the working directory with the hashes stored in a snapshot
    pub fn get_files_changed_since_snapshot<'b>(
        &self,
        snapshot: &'b Snapshot,
        hashed_files: &'b [(path::PathBuf, Hash)],
    ) -> Vec<&'b path::Path> {
        let hashed_paths = snapshot.get_files();
        self.get_changed_files(hashed_paths)
    }

    // // TODO: Move this function into the working module
    // // Takes a vector of paths or pathbufs and returns a vector of tuples containing the path and the hash
    // fn hash_file_list<P: Into<path::PathBuf>>(file_list: Vec<P>) -> Result<Vec<(path::PathBuf, Hash)>> {
    //     // TODO: This should be moved to Hash
    //     let mut path_with_hash = Vec::with_capacity(file_list.len());
    //     // Hash all the files being snapshot and store it along with the path in the snapshot structure
    //     for path_to_file in file_list {
    //         let path_to_file = path_to_file.into();
    //         let hash_string = match Hash::hash_file(path_to_file.as_path()) {
    //             Ok(hash_string) => hash_string,
    //             Err(error) => return Err(error.add_debug_message(format!("Failed to process the list of files to hash, the problem file was {}", path_to_file.display()))),
    //         };
    //         path_with_hash.push((path_to_file, hash_string));
    //     }
    //     Ok(path_with_hash)
    // }
}

impl<'a> WorkingDirectory<'a> {
    /// Given a list of paths, this function returns a list of FileMetadata
    pub(crate) fn create_metadata_list(
        files_to_store: Vec<path::PathBuf>,
    ) -> Result<Vec<FileMetadata>> {
        let mut storage_list = Vec::new();
        for file_to_store in files_to_store {
            let metadata = match WorkingDirectory::get_file_metadata(file_to_store) {
                Ok(metadata) => metadata,
                Err(error) => unimplemented!(),
            };
            storage_list.push(metadata);
        }
        Ok(storage_list)
    }

    /// Gets the file size and time modified of the given path as well as its hash
    /// This is used to collect information about a file that is part of a snapshot
    fn get_file_metadata(path_to_file: path::PathBuf) -> Result<FileMetadata> {
        let hash_of_file = match Hash::hash_file(path_to_file.as_path()) {
            Ok(hash_of_file) => hash_of_file,
            Err(error) => unimplemented!(),
        };
        let file_data = match path_to_file.metadata() {
            Ok(file_data) => file_data,
            Err(error) => unimplemented!(),
        };
        let file_size = file_data.len();
        let metadata = FileMetadata::new(
            hash_of_file,
            file_size,
            path_to_file,
            WorkingDirectory::get_modified_time(&file_data),
        );
        Ok(metadata)
    }
}

impl<'a> WorkingDirectory<'a> {
    // Given a file in the working directory what has changed since the given snapshot
    pub fn get_patch(&self, snapshot_to_compare: &Snapshot, path_to_compare: &path::Path) {
        // path_to_compare should be relative to self.path_to_working
        // Obtaining the relative path should give the path stored in the snapshot
        // Check if the given path exists in the snapshot and if LocalStorage can find it
        // Obtain a built version of the file - built here refers to uncompressed and any delta compression applied
        // QUESTION: Do we create a temp file or map the entire file into memory, can we stream the file uncompressing as we process the file
        // IDEA: Since large files should be split into multiple smaller files we should be able to process each file individually and so map an entire file at once
    }
    // TODO: This function only works for removed based overlapping sequences
    pub fn get_edits_for_remove_overlapping_sequence(
        lines_removed: &mut HashMap<String, usize>, // A map of the changes needed to the previous slice
        lines_inserted: &mut HashMap<String, usize>, // A map of the changes needed to the current slice
        lines_moved: &mut HashMap<String, LineMoved>,
        current_line: usize,                         // Current line position in the sequences
        previous_line: usize, // Contains the line where the original sequence started
        original_file: &[String], 
        edited_file: &[String], 
        new_sequence_line: usize,
        current_sequence_length: usize,
    ) -> usize {
        let mut original_length = current_sequence_length;
        let mut overlapping_length = 1;
        // New sequence previous start is at new_sequence line but don't really need it
        let original_start = current_line;
        let original_previous = previous_line;
        let overlapping_start = current_line + original_length - 1;
        let overlapping_previous = new_sequence_line;

        println!(
            "original sequence starts at {} in {:?}, and is currently {} long",
            original_start, edited_file, original_length
        );
        println!(
            "Checking new sequence, {} == {}",
            original_file[overlapping_start + overlapping_length],
            edited_file[overlapping_previous + overlapping_length]
        );
        println!(
            "Checking original sequence, {} == {}",
            edited_file[original_start + original_length],
            original_file[original_previous + original_length]
        );
        let mut original_continues = true;
        let mut overlapping_continues = true;
        loop {
            // NOTE: Once the sequence ends we must not accidently increase the length because of a random future match
            // NOTE: Can this happen, overlapping sequences need to be similiar lengths otherwise they can't overlap
            // ABCDMNJF
            // MNXLABCD
            // This can create a false match on F since the ABCD sequence ending is found at the same time

            let original_matched = match (
                edited_file.get(original_start + original_length),
                original_file.get(original_previous + original_length),
            ) {
                (Some(current_original), Some(previous_original)) => {
                    println!(
                        "Previous line was {}, Current line was {}",
                        current_original, previous_original
                    );
                    current_original == previous_original
                }
                _ => false,
            };

            let overlapping_matched = match (
                original_file.get(overlapping_start + overlapping_length),
                edited_file.get(overlapping_previous + overlapping_length),
            ) {
                (Some(second_sequence), Some(first_sequence)) => {
                    println!(
                        "Second sequence was {}, Previous sequence was {}",
                        second_sequence, first_sequence
                    );
                    second_sequence == first_sequence
                }
                _ => false,
            };

            match (
                original_continues && original_matched,
                overlapping_continues && overlapping_matched,
            ) {
                (true, true) => {
                    original_length += 1;
                    overlapping_length += 1;
                }
                (false, true) => {
                    overlapping_length += 1;
                    // Mark original sequence as completed
                    original_continues = false;
                }
                (true, false) => {
                    original_length += 1;
                    // Mark overlapping sequence as completed
                    overlapping_continues = false;
                }
                (false, false) => {
                    break;
                }
            }
        }
        println!("Length of original sequence is {}", original_length);
        println!("Length of overlapping sequence is {}", overlapping_length);
        println!("Lines Removed: {:?}", lines_removed);
        // We have lengths so now process the smaller and then return the new line position
        if original_length <= overlapping_length {
            // We add the original sequence to the changes as that is the smaller sequence
            for index in original_start..(original_start + original_length) {
                // TODO: To correctly add the original sequence we need to get the starting position when the overlap was detected
                // NOTE: We can solve the above problem by only adding items to the sequence if index >= overlapping_start

                // TODO: Need to take into account gaps in the overlapping sequence that need to be added to corrections
            }
            overlapping_length
        } else {
            // We add the overlapping sequence to the changes
            // NOTE: Need to remove any changes that were caused by the original stream before the overlapping stream started
            // TODO: We need to iterate over the larger sequence to process values accordingly
            // TODO: We use the smaller sequence index range to know what to do for a given index
            // TODO: More complicated than this, the range is overlapping_start to the max(start + length)
            // original_start + original_length - overlapping_start = The length of the original sequence but from the start of the overlapping sequence
            // TODO: Max iteration needs could take into account the max length of the file and avoid checking
            // This is the index range if original file is shorter than the end of the original sequence
            let min_size = original_file.len() - overlapping_start;
            let max_index = overlapping_length.max(original_start + original_length - overlapping_start);
            let actual_max = min_size.min(max_index);
            println!("Actual index max is {}", actual_max);
            println!("Test Iterating from {} to {}", overlapping_start, overlapping_start + actual_max);
            println!("Iterating from 0 to {}", max_index);
            for index in 0..actual_max {
                println!("Examining index {}, which is {} in the slice", index, index + overlapping_start);
                // We need to remove lines that were part of the original sequence that were added before 
                // we knew that there was an overlapping sequence
                // TODO: Remove previous entries ranging from overlapping_start (index 0) to actual_max 

                    // The line opposite the last line of the 
                    println!(
                        "Removing {} as leftover from processing simple sequence into overlap",
                        edited_file[index + overlapping_start].as_str()
                    );
                    lines_removed.remove(edited_file[index + overlapping_start].as_str());
                if index < overlapping_length {
                    let line_moved = LineMoved{
                        source_line: index + overlapping_start,
                        destination_line: overlapping_previous + index,
                    };
                    lines_moved.insert(original_file[overlapping_start + index].clone(), line_moved);
                    // TODO: Remove the old insert 
                } else {
                    println!("Do we need to add {} to the removed items", original_file[index + overlapping_start]);
                    lines_removed.insert(original_file[index + overlapping_start].clone(), index + overlapping_start);
                }
            }
            println!("Original: {:?}, Edited: {:?}", original_file, edited_file);
            println!("Moved Lines: {:?}, Lines Removed: {:?}", lines_moved, lines_removed);
            original_length
        }
    }

    

    // TODO: Tortoise and Hare approach for dealing with duplicates - cycle detection

    // This function can process both finding a previously removed line or a previously inserted line
    fn get_sequence_edits(
        current_corrections: &mut HashMap<String, usize>, // A map of the changes needed to the previous slice to make the two slices equilivant
        previous_corrections: &mut HashMap<String, usize>, // A map of the changes needed to the current slice to make the two slices equilivant
        current_line: usize,
        previous_line: usize,
        current_data: &[String], // This is the slice that contains the first instance of the data of a sequence
        previous_data: &[String], // This is the slice that contains the second instance of the data of a sequence
    ) -> usize {
        // We already know that the slice is at least 1 length so we start from 1
        let mut slice_length = 1;
        eprintln!("Current Data is {:?}", current_data);
        eprintln!("Current Corrections is {:?}", current_corrections);
        eprintln!("Previous Data is {:?}", previous_data);
        eprintln!("Previous Corrections is {:?}", previous_corrections);
        current_corrections.remove(current_data[current_line].as_str());
        // NOTE: While its relatively safe to do this it's possible this needs to be reinserted if there are overlapping sequences
        println!(
            "Previous Corrections after initial remove: {:?}",
            previous_corrections
        );
        let line_offset = current_line - previous_line;
        // NOTE: This match checks if the start of the slice is also the end of the slice
        // The ideal way of handling this is to immediately enter the below loop instead of special processing
        // This doesn't seem possible since we already know that the initial position is part of a slice so either we check twice
        // or change the order, ie remove then check next break if not
        match (
            previous_data.get(current_line), // Line opposite the start of the slice
            current_data.get(current_line + line_offset), // Line to check to see if the start of the slice is also the end of the slice
        ) {
            (Some(opposite_line), Some(future_line)) => {
                eprintln!(
                    "Opposite line was {}, future line was {}, do we add them {}",
                    opposite_line,
                    future_line,
                    opposite_line != future_line
                );
                if opposite_line != future_line {
                    // We add the opposite line when these two lines are not equal but are also not NONE
                    current_corrections.insert(previous_data[current_line].clone(), current_line);
                    // TODO: We can exit slice handling early since the slice has ended
                }
            }
            (Some(_), None) => {
                // This means that there is a line opposite the slice but there are no future lines and the slice is ending, so we must add this line to current corrections
                current_corrections.insert(previous_data[current_line].clone(), current_line);
                // TODO: We can end slice handling early since the slice has ended
            }
            _ => {}
        };
        // A slice can only be as long as the smallest file or section if we have broken the file into parts
        let total = current_data.len().max(previous_data.len()) - current_line;
        eprintln!("Processing a slice that is potentially {} long", total);

        // TODO: A loop here is not ideal but it can be vectorized
        // Process the slice
        // NOTE: We always check ahead by one to see if we add the current opposite side to the removed lines
        for _ in 1..total {
            eprintln!(
                "Slice Iteration: Previous corrections are {:?}, current corrections are {:?}",
                &current_corrections, &previous_corrections
            );
            // QUESTION: Ideally we use an iterator here, if we return None the iterator stops, but dealing with overlapping slices becomes a challenge
            // TODO: These three variables can all be moved inside a custom iterator
            // Returns true if the two lines match otherwise false, this includes a line not being present etc...
            let slice_continues = match (
                previous_data.get(previous_line + slice_length), // Previous line position
                current_data.get(current_line + slice_length),   // Current line position
            ) {
                (Some(previous_line_data), Some(current_line_data)) => {
                    println!(
                        "Previous line was {}, Current line was {}",
                        previous_line_data, current_line_data
                    );
                    previous_line_data == current_line_data
                }
                _ => false,
            };
            let add_opposite = match (
                previous_data.get(current_line + slice_length), // Line opposite the current position in slice
                current_data.get(current_line + slice_length + line_offset), // Line to check to see if slice continues in future, ie do we add opposite
            ) {
                (Some(opposite_line), Some(future_line)) => {
                    println!(
                        "Opposite line was {}, future line was {}, do we add them {}",
                        opposite_line,
                        future_line,
                        opposite_line != future_line
                    );
                    opposite_line != future_line // We add the opposite line when these two lines are not equal but are also not NONE
                }
                (Some(_), None) => {
                    // This means that there is a line opposite a slice but there are no future lines and the slice is ending, so we must add this line to removed
                    true
                }
                _ => false,
            };
            // Is there a sequence opposite the current one, we do this by seeing if we have previously seen the value opposite the current sequence listed in the previous corrections
            let overlapping_slice =
                previous_data
                    .get(current_line + slice_length)
                    .and_then(|line| {
                        // previous_corrections.get returns a reference to its data, its data type is usize and is copyable
                        // we may need to borrow previous_corrections again if we encounter overlapping sequences so we dereference
                        // the usize and return it as an option
                        previous_corrections
                            .get(line)
                            .and_then(|value| Some(*value))
                    });
            println!(
                "Overlap Check: Looked for {:?} in {:?}",
                previous_data.get(current_line + slice_length),
                previous_corrections
            );
            println!(
                "When checking for overlapping slice we found {:?} at line {}",
                overlapping_slice,
                current_line + slice_length
            );
            // We match against the next index as well if it exists
            // slice_continues, slice_continues + 1, overlapping_slice
            match (slice_continues, add_opposite, overlapping_slice) {
                (true, _, Some(line_of_overlap)) => {
                    // TODO: Here we return the longest of the two slices as the matching sequence
                    // let longest_slice = WorkingDirectory::get_edits_for_overlapping_sequence(
                    //     current_corrections,
                    //     previous_corrections,
                    //     lines
                    //     current_line,
                    //     previous_line,
                    //     current_data,
                    //     previous_data,
                    //     line_of_overlap,
                    //     slice_length + 1, // We add one to account for the current iteration through the slice
                    // );
                    unimplemented!("Debug Overlapping slices mid-sequence");
                    // return current_line + longest_slice;
                }
                (true, true, None) => {
                    // Slice continues and we add the opposite to removed
                    eprintln!(
                                            "Slice continues at {} in the new document and we need to add the opposite line",
                                            current_line + slice_length
                                        );
                    // If our position in the slice is less than the current position in the new file
                    if previous_line + slice_length < current_line {
                        // Remove items that were previously considered removed since we have not yet reached a point where previous doesn't point to unprocessed lines
                        eprintln!(
                                                "Removing previous {} from removed lines since line {} in original should be part of the slice and is less than {} which is the current line position in the new document",
                                                &previous_data[previous_line + slice_length],
                                                previous_line + slice_length,
                                                current_line
                                            );
                        current_corrections.remove(&previous_data[previous_line + slice_length]);
                    }
                    // new_line + slice_length
                    println!(
                        "Adding {} at line {} to removed lines",
                        previous_data[current_line + slice_length],
                        current_line + slice_length
                    );
                    current_corrections.insert(
                        previous_data[current_line + slice_length].clone(),
                        current_line + slice_length,
                    );
                    slice_length += 1;
                }
                (true, false, None) => {
                    // Slice continues but we do not add the opposite to removed
                    eprintln!(
                        "Slice continues at {} in the new document",
                        current_line + slice_length
                    );
                    // If our position in the slice is less than the current position in the new file
                    if previous_line + slice_length < current_line {
                        // Remove items that were previously considered removed since we have not yet reached a point where previous doesn't point to unprocessed lines
                        eprintln!(
                                                "Removing previous {} from removed lines since line {} in original should be part of the slice and is less than {} which is the current line position in the new document",
                                                &previous_data[previous_line + slice_length],
                                                previous_line + slice_length,
                                                current_line
                                            );
                        current_corrections.remove(&previous_data[previous_line + slice_length]);
                    }
                    slice_length += 1;
                }
                (false, _, _) => break, // If the original slice stops as we detect another slice we dont care we deal with that on the next iteration
            }
        }
        current_line + slice_length
    }

    // This function can process both finding a previously removed line or a previously inserted line
    fn get_insert_sequence_edits(
        lines_inserted: &mut HashMap<String, usize>, // A map of the changes needed to the previous slice to make the two slices equilivant
        lines_removed: &mut HashMap<String, usize>, // A map of the changes needed to the current slice to make the two slices equilivant
        lines_moved: &mut HashMap<String, LineMoved>,
        current_line: usize,
        previous_line: usize,
        edited_file: &[String], // This is the slice that contains the first instance of the data of a sequence
        original_file: &[String], // This is the slice that contains the second instance of the data of a sequence
    ) -> usize {
        // We already know that the slice is at least 1 length so we start from 1
        let mut slice_length = 1;
        eprintln!("Original Data is {:?}", original_file);
        eprintln!("Edited data is {:?}", edited_file);
        eprintln!("Lines inserted are {:?}", lines_inserted);
        eprintln!("Lines removed are {:?}", lines_removed);
        lines_inserted.remove(edited_file[previous_line].as_str());
        // NOTE: While its relatively safe to do this it's possible this needs to be reinserted if there are overlapping sequences
        println!(
            "Lines removed after discovering a line that was thought to be inserted: {:?}",
            lines_inserted
        );
        let line_offset = current_line - previous_line;
        // NOTE: This match checks if the start of the slice is also the end of the slice
        // The ideal way of handling this is to immediately enter the below loop instead of special processing
        // This doesn't seem possible since we already know that the initial position is part of a slice so either we check twice
        // or change the order, ie remove then check next break if not
        if WorkingDirectory::is_part_of_sequence(
            original_file,
            edited_file,
            current_line,
            line_offset,
        ) {
            lines_inserted.insert(edited_file[current_line].clone(), current_line);
        }
        // A slice can only be as long as the smallest file or section if we have broken the file into parts
        let total = original_file.len().max(edited_file.len()) - current_line;
        eprintln!("Processing a slice that is potentially {} long", total);

        // TODO: A loop here is not ideal but it can be vectorized
        // Process the slice
        // NOTE: We always check ahead by one to see if we add the current opposite side to the removed lines
        for _ in 1..total {
            eprintln!(
                "Slice Iteration: Previous corrections are {:?}, current corrections are {:?}",
                &lines_removed, &lines_inserted
            );
            // QUESTION: Ideally we use an iterator here, if we return None the iterator stops, but dealing with overlapping slices becomes a challenge
            // TODO: These three variables can all be moved inside a custom iterator
            // Returns true if the two lines match otherwise false, this includes a line not being present etc...
            let slice_continues = match (
                edited_file.get(previous_line + slice_length), // Previous line position
                original_file.get(current_line + slice_length), // Current line position
            ) {
                (Some(previous_line_data), Some(current_line_data)) => {
                    println!(
                        "Previous line was {}, Current line was {}",
                        previous_line_data, current_line_data
                    );
                    previous_line_data == current_line_data
                }
                _ => false,
            };
            let add_opposite = WorkingDirectory::is_part_of_sequence(
                original_file,
                edited_file,
                current_line + slice_length,
                line_offset,
            );
            // Is there a sequence opposite the current one, we do this by seeing if we have previously seen the value opposite the current sequence listed in the previous corrections
            let overlapping_slice = edited_file
                .get(current_line + slice_length)
                .and_then(|line| {
                    // previous_corrections.get returns a reference to its data, its data type is usize and is copyable
                    // we may need to borrow previous_corrections again if we encounter overlapping sequences so we dereference
                    // the usize and return it as an option
                    lines_inserted.get(line).and_then(|value| Some(*value))
                });
            println!(
                "Overlap Check: Looked for {:?} in {:?}",
                edited_file.get(current_line + slice_length),
                original_file
            );
            println!(
                "When checking for overlapping slice we found {:?} at line {}",
                overlapping_slice,
                current_line + slice_length
            );
            // We match against the next index as well if it exists
            // slice_continues, slice_continues + 1, overlapping_slice
            match (slice_continues, add_opposite, overlapping_slice) {
                (true, _, Some(line_of_overlap)) => {
                    // TODO: Here we return the longest of the two slices as the matching sequence
                    let longest_slice = WorkingDirectory::get_edits_for_remove_overlapping_sequence(
                        lines_removed,
                        lines_inserted,
                        lines_moved,
                        current_line,
                        previous_line,
                        original_file,
                        edited_file,
                        line_of_overlap,
                        slice_length + 1, // We add one to account for the current iteration through the slice
                    );
                    unimplemented!("Debug Overlapping slices mid-sequence");
                    return current_line + longest_slice;
                }
                (true, true, None) => {
                    // Slice continues and we add the opposite to removed
                    eprintln!(
                                            "Slice continues at {} in the new document and we need to add the opposite line",
                                            current_line + slice_length
                                        );
                    // If our position in the slice is less than the current position in the new file
                    if previous_line + slice_length < current_line {
                        // Remove items that were previously considered inserted since we have not yet reached a point where previous doesn't point to unprocessed lines
                        eprintln!(
                                                "Removing previous {} from removed lines since line {} in original should be part of the slice and is less than {} which is the current line position in the new document",
                                                &edited_file[previous_line + slice_length],
                                                previous_line + slice_length,
                                                current_line
                                            );
                        lines_inserted.remove(&edited_file[previous_line + slice_length]);
                    }
                    // new_line + slice_length
                    println!(
                        "Adding {} at line {} to inserted lines",
                        edited_file[current_line + slice_length],
                        current_line + slice_length
                    );
                    lines_inserted.insert(
                        edited_file[current_line + slice_length].clone(),
                        current_line + slice_length,
                    );
                    slice_length += 1;
                }
                (true, false, None) => {
                    // Slice continues but we do not add the opposite to removed
                    eprintln!(
                        "Slice continues at {} in the new document",
                        current_line + slice_length
                    );
                    // If our position in the slice is less than the current position in the new file
                    if previous_line + slice_length < current_line {
                        // Remove items that were previously considered removed since we have not yet reached a point where previous doesn't point to unprocessed lines
                        eprintln!(
                                                "Removing previous {} from removed lines since line {} in original should be part of the slice and is less than {} which is the current line position in the new document",
                                                &edited_file[previous_line + slice_length],
                                                previous_line + slice_length,
                                                current_line
                                            );
                        lines_inserted.remove(&edited_file[previous_line + slice_length]);
                    }
                    slice_length += 1;
                }
                (false, _, _) => break, // If the original slice stops as we detect another slice we dont care we deal with that on the next iteration
            }
        }
        current_line + slice_length
    }

    fn is_part_of_sequence(
        sequence_data: &[String],
        opposite_sequence: &[String],
        current_line_in_slice: usize,
        line_offset: usize,
    ) -> bool {
        match (
            opposite_sequence.get(current_line_in_slice), // Line opposite the current position in slice
            sequence_data.get(current_line_in_slice + line_offset), // Line to check to see if slice continues in future, ie do we add opposite
        ) {
            (Some(opposite_line), Some(future_line)) => {
                println!(
                    "Opposite line was {}, future line was {}, do we add them {}",
                    opposite_line,
                    future_line,
                    opposite_line != future_line
                );
                opposite_line != future_line // We add the opposite line when these two lines are not equal but are also not NONE
            }
            (Some(_), None) => {
                // This means that there is a line opposite a slice but there are no future lines and the slice is ending, so we must add this line to removed
                true
            }
            _ => false,
        }
    }

    // This function can process both finding a previously removed line or a previously inserted line
    fn get_remove_sequence_edits(
        lines_inserted: &mut HashMap<String, usize>, // A map of the changes needed to the previous slice to make the two slices equilivant
        lines_removed: &mut HashMap<String, usize>, // A map of the changes needed to the current slice to make the two slices equilivant
        lines_moved: &mut HashMap<String, LineMoved>,
        current_line: usize,
        previous_line: usize,
        edited_file: &[String], // This is the slice that contains the first instance of the data of a sequence
        original_file: &[String], // This is the slice that contains the second instance of the data of a sequence
    ) -> usize {
        // We already know that the slice is at least 1 length so we start from 1
        let mut slice_length = 1;
        eprintln!("Original Data is {:?}", original_file);
        eprintln!("Edited data is {:?}", edited_file);
        eprintln!("Lines inserted are {:?}", lines_inserted);
        eprintln!("Lines removed are {:?}", lines_removed);
        lines_removed.remove(original_file[previous_line].as_str());
        // NOTE: While its relatively safe to do this it's possible this needs to be reinserted if there are overlapping sequences
        println!(
            "Lines removed after discovering a line that was tought to be removed: {:?}",
            lines_removed
        );
        let line_offset = current_line - previous_line;
        // NOTE: This match checks if the start of the slice is also the end of the slice
        // The ideal way of handling this is to immediately enter the below loop instead of special processing
        // This doesn't seem possible since we already know that the initial position is part of a slice so either we check twice
        // or change the order, ie remove then check next break if not
        if WorkingDirectory::is_part_of_sequence(
            edited_file,
            original_file,
            current_line,
            line_offset,
        ) {
            lines_removed.insert(original_file[current_line].clone(), current_line);
            // TODO: If we end up here then the slice had a length of 1 and we are done
        }

        // A slice can only be as long as the smallest file or section if we have broken the file into parts
        let total = original_file.len().max(edited_file.len()) - current_line;
        eprintln!("Processing a slice that is potentially {} long", total);

        // TODO: A loop here is not ideal but it can be vectorized
        // Process the slice
        // NOTE: We always check ahead by one to see if we add the current opposite side to the removed lines
        for _ in 1..total {
            eprintln!(
                "Slice Iteration: Previous corrections are {:?}, current corrections are {:?}",
                &lines_removed, &lines_inserted
            );
            // QUESTION: Ideally we use an iterator here, if we return None the iterator stops, but dealing with overlapping slices becomes a challenge
            // TODO: These three variables can all be moved inside a custom iterator
            // Returns true if the two lines match otherwise false, this includes a line not being present etc...
            let slice_continues = match (
                original_file.get(previous_line + slice_length), // Previous line position
                edited_file.get(current_line + slice_length),    // Current line position
            ) {
                (Some(previous_line_data), Some(current_line_data)) => {
                    println!(
                        "Previous line was {}, Current line was {}",
                        previous_line_data, current_line_data
                    );
                    previous_line_data == current_line_data
                }
                _ => false,
            };
            let add_opposite = WorkingDirectory::is_part_of_sequence(
                edited_file,
                original_file,
                current_line + slice_length,
                line_offset,
            );
            // Is there a sequence opposite the current one, we do this by seeing if we have previously seen the value opposite the current sequence listed in the previous corrections
            let overlapping_slice =
                original_file
                    .get(current_line + slice_length)
                    .and_then(|line| {
                        // previous_corrections.get returns a reference to its data, its data type is usize and is copyable
                        // we may need to borrow previous_corrections again if we encounter overlapping sequences so we dereference
                        // the usize and return it as an option
                        lines_inserted.get(line).and_then(|value| Some(*value))
                    });
            println!(
                "Overlap Check: Looked for {:?} in {:?}",
                edited_file.get(current_line + slice_length),
                original_file
            );
            println!(
                "When checking for overlapping slice we found {:?} at line {}",
                overlapping_slice,
                current_line + slice_length
            );
            // We match against the next index as well if it exists
            // slice_continues, slice_continues + 1, overlapping_slice
            match (slice_continues, add_opposite, overlapping_slice) {
                (true, _, Some(line_of_overlap)) => {
                    // TODO: Here we return the longest of the two slices as the matching sequence
                    let longest_slice = WorkingDirectory::get_edits_for_remove_overlapping_sequence(
                        lines_removed,
                        lines_inserted,
                        lines_moved,
                        current_line,
                        previous_line,
                        original_file,
                        edited_file,
                        line_of_overlap,
                        slice_length + 1, // We add one to account for the current iteration through the slice
                    );
                    //unimplemented!("Debug Overlapping slices mid-sequence");
                    return current_line + longest_slice;
                }
                (true, true, None) => {
                    // Slice continues and we add the opposite to removed
                    eprintln!(
                                            "Slice continues at {} in the new document and we need to add the opposite line",
                                            current_line + slice_length
                                        );
                    // If our position in the slice is less than the current position in the new file
                    if previous_line + slice_length < current_line {
                        // Remove items that were previously considered removed since we have not yet reached a point where previous doesn't point to unprocessed lines
                        eprintln!(
                                                "Removing previous {} from removed lines since line {} in original should be part of the slice and is less than {} which is the current line position in the new document",
                                                &original_file[previous_line + slice_length],
                                                previous_line + slice_length,
                                                current_line
                                            );
                        lines_removed.remove(&original_file[previous_line + slice_length]);
                    }
                    // new_line + slice_length
                    eprintln!(
                        "Adding {} at line {} to removed lines",
                        original_file[current_line + slice_length],
                        current_line + slice_length
                    );
                    lines_removed.insert(
                        original_file[current_line + slice_length].clone(),
                        current_line + slice_length,
                    );
                    slice_length += 1;
                }
                (true, false, None) => {
                    // Slice continues but we do not add the opposite to removed
                    eprintln!(
                        "Slice continues at {} in the new document",
                        current_line + slice_length
                    );
                    // If our position in the slice is less than the current position in the new file
                    if previous_line + slice_length < current_line {
                        // Remove items that were previously considered removed since we have not yet reached a point where previous doesn't point to unprocessed lines
                        eprintln!(
                                                "Removing previous {} from removed lines since line {} in original should be part of the slice and is less than {} which is the current line position in the new document",
                                                &original_file[previous_line + slice_length],
                                                previous_line + slice_length,
                                                current_line
                                            );
                        lines_removed.remove(&original_file[previous_line + slice_length]);
                    }
                    slice_length += 1;
                }
                (false, _, _) => break, // If the original slice stops as we detect another slice we dont care we deal with that on the next iteration
            }
        }
        current_line + slice_length
    }

    #[cfg(test)]
    pub fn file_patch(
        original_path: &path::Path,
        edited_path: &path::Path,
    ) -> (Vec<usize>, Vec<usize>) {
        use rand::Rng;
        use std::hash::Hasher;

        // TODO: Just for testing diffing algorithms
        let mut original_data = fs::OpenOptions::new()
            .read(true)
            .open(original_path)
            .unwrap();
        let new_data = fs::OpenOptions::new().read(true).open(edited_path).unwrap();
        let original_reader = io::BufReader::new(original_data);
        let new_reader = io::BufReader::new(new_data);
        let original_lines = original_reader.lines();
        let new_lines = new_reader.lines();

        // let kl = original_lines.zip(new_lines);

        // TODO: Need to hash the lines
        let mut rng = rand::thread_rng();
        let key1: u128 = rng.gen();
        let key2: u128 = rng.gen();
        let hasher = ahash::AHasher::new_with_keys(key1, key2);
        let original_file: Vec<String> = original_lines
            .map(|line| {
                line.unwrap()
                // TODO: We need to trim all spaces
                // let mut hash = ahash::AHasher::new_with_keys(valid_line.len() as u64, key1);
                // hash.write(valid_line.as_bytes());
                // hash.finish()
            })
            .collect();
        let edited_file: Vec<String> = new_lines
            .map(|line| {
                line.unwrap()
                // let mut hash = ahash::AHasher::new_with_keys(valid_line.len() as u64, key1);
                // hash.write(valid_line.as_bytes());
                // hash.finish()
            })
            .collect();
        println!("Original File {:?}", original_file);
        println!("Edited File {:?}", edited_file);

        let mut lines_inserted = HashMap::new();
        let mut lines_removed = HashMap::new();
        let mut lines_moved = HashMap::new();
        // Lines that were thought to have been removed but were moved
        // Lines that were thought to have been inserted but were moved
        // We only use the results for each section we process from one of the above move hash maps
        let mut current_line: usize = 0;
        // TODO: Do we replace this loop with an iterator
        // NOTE: We can precompute lines to process by max file - min file
        // NOTE: Then we can process the remainder in a following match statement and merge the results
        // NOTE: if max_file = original then process original rem
        // NOTE: if max_file = edited then process edited rem
        // TODO: Simplify this to an iterator followed by another iterator, iterate over all lines in both files and then lines present in one of the files
        let total_lines = original_file.len().min(edited_file.len());
        // Cant iterate over the lines directly because of overlapping sequences
        // Iterating over lines for single sequences would be possible with a custom iterator but gains us nothing
        while current_line < total_lines {
            
            println!("Scanning line {} in both files", current_line);
            println!(
                "Original: {} Edited: {}",
                original_file[current_line], edited_file[current_line]
            );
            if original_file[current_line] == edited_file[current_line] {
                // Matching lines are trivial as there is nothing to do so move to next line
                current_line += 1;
            } else {
                // Compares the line from the original text with any previous unmatched lines from the new text and vice versa
                // NOTE: Lines that were considered removed will be found in new_data since removed lines were present in original but not in changed
                // NOTE: Lines that were considered inserted will be found in original since inserted lines were present in changed but not in original
                match (
                    lines_removed.get(&edited_file[current_line]),
                    lines_inserted.get(original_file[current_line].as_str()),
                ) {
                    (Some(previously_removed), Some(previously_inserted)) => {
                        // Both lines have been seen so two slices are overlapping
                        println!("Both have been seen so guarenteed overlapping move");
                        unimplemented!("Overlapping sequences not supported yet");
                        //return WorkingDirectory::process_overlapping(6);
                        // We check both slices until one of them ends, the one that ends first is the one we use
                        // Here the two overlapping slices begin at the same point
                        current_line += 1;
                    }
                    (Some(previously_removed), None) => {
                        // Example of match
                        // A K
                        // B L
                        // C M
                        // D A <- Previously thought to be removed

                        // A line in the original file has been seen before
                        // Get the line number where we previously saw this line
                        let previous_line = *previously_removed;
                        eprintln!("A line previously thought to be removed has been seen, {} has been seen at line {}, {} has not", edited_file[current_line].as_str(), previous_line, original_file[current_line].as_str());
                        println!("Line before sequence: {}", current_line);
                        current_line = WorkingDirectory::get_remove_sequence_edits(
                            &mut lines_inserted,
                            &mut lines_removed,
                            &mut lines_moved,
                            current_line,
                            previous_line,
                            edited_file.as_slice(),
                            original_file.as_slice(),
                        );
                        //num_iter.skip(new_line);
                        //num_iter.next();
                        println!("Line after sequence: {}", current_line);
                    }
                    (None, Some(previously_inserted)) => {
                        // Example of match
                        // A K
                        // B L
                        // C M
                        // D A <- Previously thought to be removed

                        // A line in the original file has been seen before
                        // Get the line number where we previously saw this line
                        let previous_line = *previously_inserted;
                        eprintln!("A line previously thought to be inserted has been seen, {} has been seen at line {}, {} has not", original_file[current_line].as_str(), previous_line, edited_file[current_line].as_str());
                        current_line = WorkingDirectory::get_insert_sequence_edits(
                            &mut lines_inserted,
                            &mut lines_removed,
                            &mut lines_moved,
                            current_line,
                            previous_line,
                            edited_file.as_slice(),
                            original_file.as_slice(),
                        );
                    }
                    (None, None) => {
                        // Neither of these lines have been seen before
                        // So we they are prospective removed and inserted lines
                        println!(
                            "We have not seen {} at line {} or {} at line {} before",
                            &original_file[current_line],
                            current_line,
                            &edited_file[current_line],
                            current_line
                        );
                        // This does not mean that these lines are not duplicates
                        lines_inserted.insert(edited_file[current_line].clone(), current_line);
                        lines_removed.insert(original_file[current_line].clone(), current_line);
                        current_line += 1;
                    }
                }
                println!("----------------------------------------------------------------------------------------------------");
            }
        }
        // TODO: Currently the above function can process a slice that exceeds the length of one of the files
        // TODO: But only if the slice starts within the range of both files
        println!("Current line is {}", current_line);
        let appended_lines = current_line..edited_file.len().max(original_file.len());
        if original_file.len() >= edited_file.len() {
            // Scan additional lines in original file
            for current_line in appended_lines {
                println!("Scanning line {} in original lines", current_line);
                println!(
                    "Additional Line: {}",
                    original_file[current_line]
                );
                if lines_inserted.contains_key(&original_file[current_line]) {
                    // This is part of a slice ie
                    // ABCJUI
                    // HFJXZKABC
                    // However do we really save anything by scanning ahead instead of processing one at a time
                    println!("Removing {} at line {} at the end of original file", original_file[current_line], current_line);
                    lines_inserted
                        .remove_entry(&original_file[current_line]);
                } else {
                    lines_removed.insert(edited_file[current_line].clone(), current_line);
                }
            }
        } else {
            // Scan additional lines in edited
            for current_line in appended_lines {
                println!("Scanning line {} in edited lines", current_line);
                println!(
                    "Additional Line: {}",
                    edited_file[current_line]
                );
                if lines_removed.contains_key(&edited_file[current_line]) {
                    // This is part of a slice ie
                    // ABCJUI
                    // HFJXZKABC
                    // However do we really save anything by scanning ahead instead of processing one at a time
                    println!("Removing {} at line {} at the end of edited file", edited_file[current_line], current_line);
                    lines_removed
                        .remove_entry(&edited_file[current_line]);
                } else {
                    lines_inserted.insert(edited_file[current_line].clone(), current_line);
                }
            }
        }
        

        println!("Inserted {:?}", lines_inserted);
        println!("Lines removed {:?}", lines_removed);
        return (
            lines_removed
                .values()
                .map(|line_number| *line_number)
                .collect::<Vec<_>>(),
            lines_inserted
                .values()
                .map(|line_number| *line_number)
                .collect::<Vec<_>>(),
        );
    }

    fn infinite_loop_method(
        current_line: usize,
        original_file: &mut Vec<String>,
        edited_file: &mut Vec<String>,
    ) {
        // loop {
        //     // TODO: Remove this initial branch and instead compute the total iterations required as well as the additional reads required at the end for the longer file or use an iterator that returns None when the file has no more data
        //     if current_line < original_file.len() && current_line < edited_file.len() {
        //         if original_file[current_line] == edited_file[current_line] {
        //             // Lines are the same, just move to the next set of lines
        //             current_line += 1;
        //         } else {
        //             // Compares the line from the original text with any previous unmatched lines from the new text and vice versa
        //             // NOTE: Lines that were considered removed will be found in new_data since removed lines were present in original but not in changed
        //             // NOTE: Lines that were considered inserted will be found in original since inserted lines were present in changed but not in original
        //             match (
        //                 lines_removed.get(&edited_file[current_line]),
        //                 lines_inserted.get(original_file[current_line].as_str()),
        //             ) {
        //                 (Some(previously_removed), Some(previously_inserted)) => {
        //                     // Both lines have been seen so two slices are overlapping
        //                     println!("Both have been seen so guarenteed overlapping move");
        //                     unimplemented!("Overlapping sequences not supported yet");
        //                     //return WorkingDirectory::process_overlapping(6);
        //                     // We check both slices until one of them ends, the one that ends first is the one we use
        //                     // Here the two overlapping slices begin at the same point
        //                     current_line += 1;
        //                 }
        //                 (Some(previously_removed), None) => {
        //                     // Example of match
        //                     // A K
        //                     // B L
        //                     // C M
        //                     // D A <- Previously thought to be removed

        //                     // A line in the original file has been seen before
        //                     // Get the line number where we previously saw this line
        //                     let previous_line = *previously_removed;
        //                     eprintln!("A line previously thought to be removed has been seen, {} has been seen at line {}, {} has not", edited_file[current_line].as_str(), previous_line, original_file[current_line].as_str());
        //                     current_line = WorkingDirectory::get_remove_sequence_edits(
        //                         &mut lines_inserted,
        //                         &mut lines_removed,
        //                         current_line,
        //                         previous_line,
        //                         edited_file.as_slice(),
        //                         original_file.as_slice(),
        //                     );
        //                 }
        //                 (None, Some(previously_inserted)) => {
        //                     // Example of match
        //                     // A K
        //                     // B L
        //                     // C M
        //                     // D A <- Previously thought to be removed

        //                     // A line in the original file has been seen before
        //                     // Get the line number where we previously saw this line
        //                     let previous_line = *previously_inserted;
        //                     eprintln!("A line previously thought to be inserted has been seen, {} has been seen at line {}, {} has not", original_file[current_line].as_str(), previous_line, edited_file[current_line].as_str());
        //                     current_line = WorkingDirectory::get_insert_sequence_edits(
        //                         &mut lines_inserted,
        //                         &mut lines_removed,
        //                         current_line,
        //                         previous_line,
        //                         edited_file.as_slice(),
        //                         original_file.as_slice(),
        //                     );
        //                 }
        //                 (None, None) => {
        //                     // Neither of these lines have been seen before
        //                     // So we they are prospective removed and inserted lines
        //                     println!(
        //                         "We have not seen {} at line {} or {} at line {} before",
        //                         &original_file[current_line],
        //                         current_line,
        //                         &edited_file[current_line],
        //                         current_line
        //                     );
        //                     // This does not mean that these lines are not duplicates
        //                     // Check for a new_data[new_line] in lines_inserted and if so edit the entry with the an additional line
        //                     lines_inserted.insert(edited_file[current_line].clone(), current_line);
        //                     lines_removed.insert(original_file[current_line].clone(), current_line);
        //                     current_line += 1;
        //                 }
        //             }
        //             println!("----------------------------------------------------------------------------------------------------");
        //         }
        //     } else if current_line < original_file.len() {
        //         // We have run out of lines in the new file
        //         // Check original line to see if it is listed in the inserted lines = becomes moved
        //         if lines_inserted.contains_key(&original_file[current_line]) {
        //             // This was a move and not a insert
        //             // TODO: We can still have slices
        //             let (previous_insert, previous_insert_line) = lines_inserted
        //                 .remove_entry(&original_file[current_line])
        //                 .unwrap();
        //             println!("Line {} was moved not inserted", current_line);
        //             lines_inserted_move
        //                 .insert(previous_insert, (current_line, previous_insert_line));
        //         } else {
        //             lines_removed.insert(original_file[current_line].clone(), current_line);
        //         }
        //         current_line += 1;
        //     } else if current_line < edited_file.len() {
        //         // We have run out of lines in the old file
        //         // Check new line to see if it is listed in the removed lines = becomes moved and check the index difference for alignment
        //         if lines_removed.contains_key(&edited_file[current_line]) {
        //             // TODO: We can still have slices
        //             // We need to check for overlapping slices, otherwise we ignore them - but they can never overlap
        //             // This was a move and not a removal
        //             let (previous_remove, previous_remove_line) = lines_removed
        //                 .remove_entry(&edited_file[current_line])
        //                 .unwrap();
        //             println!("Line {} was moved not removed", previous_remove_line);
        //             lines_removed_move
        //                 .insert(previous_remove, (previous_remove_line, current_line));
        //         } else {
        //             lines_inserted.insert(edited_file[current_line].clone(), current_line);
        //         }
        //         current_line += 1;
        //     // Add new to inserted
        //     } else {
        //         // Nothing left to scan
        //         break;
        //     }
        // }
    }
}

enum SliceType {
    Overlapping(usize, usize),
    Simple(usize),
}

#[cfg(test)]
mod tests {
    use super::RawZip;
    use super::{Move, ProspectiveDifference, ProspectiveMove};
    use crate::hash::Hash;
    use crate::working::WorkingDirectory;
    use smallvec::SmallVec;
    use std::collections::HashMap;
    use testspace::Alphabet;
    use testspace::{TestSpace, TestSpaceFile};

    #[test]
    fn basic_previously_removed_short_test() {
        let ts = TestSpace::new();
        let mut original_file = ts.create_text_file();
        original_file.append_line("A"); // 0
        original_file.append_line("B"); // 1
        original_file.append_line("C"); // 2
        original_file.append_line("D"); // 3
        let mut changed_file = ts.create_text_file();
        changed_file.append_line("J"); // 0
        changed_file.append_line("A"); // 1
        changed_file.append_line("B"); // 2
        changed_file.append_line("C"); // 3
                                       // ABCD
                                       // JABC
        let (mut removed_lines, mut inserted_lines) =
            WorkingDirectory::file_patch(original_file.get_path(), changed_file.get_path());
        removed_lines.sort();
        inserted_lines.sort();
        assert_eq!(removed_lines.len(), 1);
        assert_eq!(inserted_lines.len(), 1);
        assert_eq!(removed_lines, vec!(3)); // Line 3 in original document was removed
        assert_eq!(inserted_lines, vec!(0)); // Line 0 in edited document was inserted
    }

    #[test]
    fn basic_previously_removed_test() {
        let ts = TestSpace::new();
        let mut original_file = ts.create_text_file();
        original_file.append_line("A"); // 0
        original_file.append_line("B"); // 1
        original_file.append_line("C"); // 2
        original_file.append_line("D"); // 3
        original_file.append_line("E"); // 4
        let mut changed_file = ts.create_text_file();
        changed_file.append_line("J"); // 0
        changed_file.append_line("K"); // 1
        changed_file.append_line("A"); // 2
        changed_file.append_line("B"); // 3
        changed_file.append_line("C"); // 4
                                       // ABCDE
                                       // JKABC
        let (mut removed_lines, mut inserted_lines) =
            WorkingDirectory::file_patch(original_file.get_path(), changed_file.get_path());
        removed_lines.sort();
        inserted_lines.sort();
        assert_eq!(removed_lines, vec!(3, 4)); // Line 3 and 4 in original document was removed
        assert_eq!(inserted_lines, vec!(0, 1)); // Line 0 and 1 in edited document was inserted
    }

    #[test]
    fn length_of_one_sequence_removed_test() {
        // Sequence is only one long
        let ts = TestSpace::new();
        let mut original_file = ts.create_text_file();
        original_file.append_line("A"); // 0
        original_file.append_line("B"); // 1
        original_file.append_line("C"); // 2
        original_file.append_line("P"); // 3
        let mut changed_file = ts.create_text_file();
        changed_file.append_line("K"); // 0
        changed_file.append_line("A"); // 1
        changed_file.append_line("S"); // 2
        changed_file.append_line("T"); // 3
                                       // ABCP
                                       // KAST
        let (mut removed_lines, mut inserted_lines) =
            WorkingDirectory::file_patch(original_file.get_path(), changed_file.get_path());
        removed_lines.sort();
        inserted_lines.sort();
        assert_eq!(removed_lines, vec!(1, 2, 3)); // Line 3 and 4 in original document was removed
        assert_eq!(inserted_lines, vec!(0, 2, 3)); // Line 0 and 1 in edited document was inserted
    }

    #[test]
    fn basic_previously_removed_more_overlap_test() {
        // Tests appropriate response to multiple overlapping lines
        // ie A and B in changed file overlap with sequence in original in 2 places not one
        let ts = TestSpace::new();
        let mut original_file = ts.create_text_file();
        original_file.append_line("A"); // 0
        original_file.append_line("B"); // 1
        original_file.append_line("C"); // 2
        original_file.append_line("D"); // 3
        original_file.append_line("E"); // 4
        let mut changed_file = ts.create_text_file();
        changed_file.append_line("J"); // 0
        changed_file.append_line("A"); // 1
        changed_file.append_line("B"); // 2
        changed_file.append_line("C"); // 3
        changed_file.append_line("F"); // 4
                                       // ABCDE
                                       // JABCF
        let (mut removed_lines, mut inserted_lines) =
            WorkingDirectory::file_patch(original_file.get_path(), changed_file.get_path());
        removed_lines.sort();
        inserted_lines.sort();
        assert_eq!(removed_lines, vec!(3, 4)); // Line 3 and 4 in original document was removed
        assert_eq!(inserted_lines, vec!(0, 4)); // Line 0 and 1 in edited document was inserted
    }

    #[test]
    fn length_of_one_sequence_inserted_test() {
        let ts = TestSpace::new();
        let mut original_file = ts.create_text_file();
        original_file.append_line("B"); // 0
        original_file.append_line("A"); // 1
        original_file.append_line("C"); // 2
        original_file.append_line("P"); // 3
        let mut changed_file = ts.create_text_file();
        changed_file.append_line("A"); // 0
        changed_file.append_line("K"); // 1
        changed_file.append_line("S"); // 2
        changed_file.append_line("T"); // 3
                                       // ABCP
                                       // KAST
        let (mut removed_lines, mut inserted_lines) =
            WorkingDirectory::file_patch(original_file.get_path(), changed_file.get_path());
        removed_lines.sort();
        inserted_lines.sort();
        assert_eq!(removed_lines, vec!(0, 2, 3));
        assert_eq!(inserted_lines, vec!(1, 2, 3));
    }

    #[test]
    fn non_interacting_sequence_test() {
        let ts = TestSpace::new();
        let mut original_file = ts.create_text_file();
        original_file.append_line("A"); // 0
        original_file.append_line("B"); // 1
        original_file.append_line("C"); // 2
        original_file.append_line("D"); // 3
        original_file.append_line("G"); // 4
        original_file.append_line("H"); // 5
        original_file.append_line("I"); // 6

        let mut changed_file = ts.create_text_file();
        changed_file.append_line("Y"); // 0
        changed_file.append_line("V"); // 1
        changed_file.append_line("Z"); // 2
        changed_file.append_line("X"); // 3
        changed_file.append_line("A"); // 4
        changed_file.append_line("B"); // 5
        changed_file.append_line("C"); // 6
        let (mut removed_lines, mut inserted_lines) =
            WorkingDirectory::file_patch(original_file.get_path(), changed_file.get_path());
        removed_lines.sort();
        inserted_lines.sort();
        assert_eq!(removed_lines, vec!(3, 4, 5, 6));
        assert_eq!(inserted_lines, vec!(0, 1, 2, 3));
    }

    #[test]
    fn basic_previously_inserted_test() {
        let ts = TestSpace::new();
        let mut original_file = ts.create_text_file();
        original_file.append_line("J"); // 0
        original_file.append_line("K"); // 1
        original_file.append_line("A"); // 2
        original_file.append_line("B"); // 3
        original_file.append_line("C"); // 4
        let mut changed_file = ts.create_text_file();
        changed_file.append_line("A"); // 0
        changed_file.append_line("B"); // 1
        changed_file.append_line("C"); // 2
        changed_file.append_line("D"); // 3
        changed_file.append_line("E"); // 4
                                       // JKABC
                                       // ABCDE
        let (mut removed_lines, mut inserted_lines) =
            WorkingDirectory::file_patch(original_file.get_path(), changed_file.get_path());
        removed_lines.sort();
        inserted_lines.sort();
        assert_eq!(removed_lines, vec!(0, 1));
        assert_eq!(inserted_lines, vec!(3, 4));
    }

    #[test]
    fn removed_uneven_lengths_test() {
        // NOTE: This uses the removed path because the slice is opposite items that would have maybe been removed
        let ts = TestSpace::new();
        let mut original_file = ts.create_text_file();
        original_file.append_line("A"); // 0
        original_file.append_line("B"); // 1
        original_file.append_line("C"); // 2
        let mut changed_file = ts.create_text_file();
        changed_file.append_line("D"); // 0
        changed_file.append_line("E"); // 1
        changed_file.append_line("A"); // 2
        changed_file.append_line("B"); // 3
        changed_file.append_line("C"); // 4
                                       // ABC
                                       // DEABC
        let (mut removed_lines, mut inserted_lines) =
            WorkingDirectory::file_patch(original_file.get_path(), changed_file.get_path());
        removed_lines.sort();
        inserted_lines.sort();
        assert_eq!(removed_lines.len(), 0);
        assert_eq!(inserted_lines, vec!(0, 1));
    }

    #[test]
    fn inserted_uneven_lengths_test() {
        let ts = TestSpace::new();
        let mut original_file = ts.create_text_file();
        original_file.append_line("D"); // 0
        original_file.append_line("E"); // 1
        original_file.append_line("A"); // 2
        original_file.append_line("B"); // 3
        original_file.append_line("C"); // 4
        let mut changed_file = ts.create_text_file();
        changed_file.append_line("A"); // 0
        changed_file.append_line("B"); // 1
        changed_file.append_line("C"); // 2
                                       // ABC
                                       // DEABC
        let (mut removed_lines, mut inserted_lines) =
            WorkingDirectory::file_patch(original_file.get_path(), changed_file.get_path());
        removed_lines.sort();
        inserted_lines.sort();
        assert_eq!(inserted_lines.len(), 0);
        assert_eq!(removed_lines, vec!(0, 1));
    }

    #[test]
    fn advanced_uneven_lengths_test() {
        // Tests the case where the sequence does not overlap with the other file
        let ts = TestSpace::new();
        let mut original_file = ts.create_text_file();
        original_file.append_line("D"); // 0
        original_file.append_line("E"); // 1
        original_file.append_line("F"); // 1
        original_file.append_line("A"); // 2
        original_file.append_line("B"); // 3
        original_file.append_line("C"); // 4
        let mut changed_file = ts.create_text_file();
        changed_file.append_line("A"); // 0
        changed_file.append_line("B"); // 1
        changed_file.append_line("C"); // 2
                                       // ABC
                                       // DEFABC
        let (mut removed_lines, mut inserted_lines) =
            WorkingDirectory::file_patch(original_file.get_path(), changed_file.get_path());
        removed_lines.sort();
        inserted_lines.sort();
        assert_eq!(inserted_lines.len(), 0);
        assert_eq!(removed_lines, vec!(0, 1, 2));
    }

    #[test]
    fn basic_overlapping_midsequence_test() {
        let ts = TestSpace::new();
        let mut original_file = ts.create_text_file();
        original_file.append_line("A"); // 0
        original_file.append_line("B"); // 1
        original_file.append_line("C"); // 2
        original_file.append_line("M"); // 3 = Overlapping sequence starts here
        original_file.append_line("N"); // 4
        let mut changed_file = ts.create_text_file();
        changed_file.append_line("M"); // 0
        changed_file.append_line("N"); // 1
        changed_file.append_line("A"); // 2
        changed_file.append_line("B"); // 3
        changed_file.append_line("C"); // 4
                                       // JKABC
                                       // ABCDE
                                       // This overlapping sequence is triggered mid sequence
        WorkingDirectory::file_patch(original_file.get_path(), changed_file.get_path());
    }

    #[test]
    fn advanced_overlapping_midsequence_test() {
        let ts = TestSpace::new();
        let mut original_file = ts.create_text_file();
        original_file.append_line("A"); // 0
        original_file.append_line("B"); // 1
        original_file.append_line("C"); // 2
        original_file.append_line("D"); // 3
        original_file.append_line("E"); // 4
        original_file.append_line("F"); // 5
        original_file.append_line("M"); // 6
        original_file.append_line("N"); // 7
        original_file.append_line("X"); // 8
        original_file.append_line("Y"); // 9
        let mut changed_file = ts.create_text_file();
        changed_file.append_line("M"); // 0
        changed_file.append_line("N"); // 1
        changed_file.append_line("O"); // 2
        changed_file.append_line("P"); // 3
        changed_file.append_line("Q"); // 4
        changed_file.append_line("A"); // 5
        changed_file.append_line("B"); // 6  = Overlapping sequence starts here
        changed_file.append_line("C"); // 7
        changed_file.append_line("D"); // 8
        changed_file.append_line("E"); // 9
        changed_file.append_line("F"); // 10
        // Should check indexes from 6..9
                                       // ABCDEFMNXY
                                       // MNOPQABCDEF
                                       // This overlapping sequence is triggered mid sequence
        WorkingDirectory::file_patch(original_file.get_path(), changed_file.get_path());
    }

    // TODO: Lots more tests for overlapping sequences

    #[test]
    fn basic_overlapping_sequence_test() {
        let ts = TestSpace::new();
        let mut original_file = ts.create_text_file();
        original_file.append_line("A"); // 0
        original_file.append_line("B"); // 1
        original_file.append_line("C"); // 2
        original_file.append_line("M"); // 3 = Overlapping sequence starts here
        original_file.append_line("N"); // 4
        original_file.append_line("O"); // 5
        let mut changed_file = ts.create_text_file();
        changed_file.append_line("M"); // 0
        changed_file.append_line("N"); // 1
        changed_file.append_line("O"); // 2
        changed_file.append_line("A"); // 3
        changed_file.append_line("B"); // 4
        changed_file.append_line("C"); // 5
                                       // JKABC
                                       // ABCDE
                                       // This overlapping sequence is triggered on a new line
        WorkingDirectory::file_patch(original_file.get_path(), changed_file.get_path());
    }

    #[test]
    fn advanced_sequence_diff_test() {
        let ts = TestSpace::new();
        let mut original_file = ts.create_text_file();
        original_file.append_line("A"); // 0
        original_file.append_line("B"); // 1
        original_file.append_line("C"); // 2
        original_file.append_line("D"); // 3
        original_file.append_line("E"); // 4
        original_file.append_line("F"); // 5
        original_file.append_line("G"); // 6
        original_file.append_line("H"); // 7
        original_file.append_line("I"); // 8
        original_file.append_line("J"); // 9
        original_file.append_line("K"); // 10
        let mut new_file = original_file.create_copy();
        // ABCDEFGHIJK =>
        // JKEABCDFGHI
        // Results should be E Moved and JK Moved
        new_file.move_line(4, 0);
        new_file.move_line(9, 0);
        new_file.move_line(10, 1);
        // WorkingDirectory::file_patch()
        WorkingDirectory::file_patch(original_file.get_path(), new_file.get_path());
    }

    #[test]
    fn diff_test() {
        let ts = TestSpace::new().allow_cleanup(false);
        let mut original_file = ts.create_text_file();
        original_file.append_line("A");
        original_file.append_line("B");
        original_file.append_line("C");
        original_file.append_line("D");
        let mut new_file = original_file.create_copy();
        // ABCD => BACD
        new_file.swap_lines(0, 1);
        // WorkingDirectory::file_patch()
        WorkingDirectory::file_patch(original_file.get_path(), new_file.get_path());
    }

    #[test]
    fn diff_test_2() {
        let ts = TestSpace::new().allow_cleanup(false);
        let mut original_file = ts.create_text_file();
        original_file.append_line("A");
        original_file.append_line("B");
        original_file.append_line("C");
        original_file.append_line("D");
        let mut new_file = original_file.create_copy();
        new_file.swap_lines(1, 3);
        WorkingDirectory::file_patch(original_file.get_path(), new_file.get_path());
    }

    #[test]
    fn diff_test_3() {
        let ts = TestSpace::new().allow_cleanup(false);
        let mut original_file = ts.create_text_file();
        original_file.append_line("A");
        original_file.append_line("B");
        original_file.append_line("C");
        original_file.append_line("D");
        let mut new_file = original_file.create_copy();
        new_file.move_line(1, 3); // A C D B
        WorkingDirectory::file_patch(original_file.get_path(), new_file.get_path());
    }

    #[test]
    fn diff_test_4() {
        let ts = TestSpace::new().allow_cleanup(false);
        let mut original_file = ts.create_text_file();
        original_file.append_line("A");
        original_file.append_line("C");
        original_file.append_line("D");
        original_file.append_line("B");
        let mut new_file = original_file.create_copy();
        new_file.move_line(3, 1); // A B C D
                                  // WorkingDirectory::file_patch()
        WorkingDirectory::file_patch(original_file.get_path(), new_file.get_path());
    }

    #[test]
    fn diff_test_5() {
        let ts = TestSpace::new().allow_cleanup(false);
        let mut original_file = ts.create_text_file();
        original_file.append_line("A line was removed");
        original_file.append_line("C wasn't moved");
        original_file.append_line("D wasn't moved either");
        original_file.append_line("B was moved");
        let mut new_file = original_file.create_copy();
        new_file.remove_line(0); // C D B
        new_file.move_line(2, 0); // B C D
                                  // WorkingDirectory::file_patch()
        WorkingDirectory::file_patch(original_file.get_path(), new_file.get_path());
    }

    #[test]
    fn diff_test_6() {
        let ts = TestSpace::new().allow_cleanup(false);
        let mut original_file = ts.create_text_file();
        original_file.append_line("A");
        original_file.append_line("B");
        original_file.append_line("C");
        original_file.append_line("D");
        let mut new_file = original_file.create_copy();

        new_file.move_line(2, 1); // ACBD
        new_file.insert_line(3, "A"); // ACBAD
                                      // WorkingDirectory::file_patch()
        WorkingDirectory::file_patch(original_file.get_path(), new_file.get_path());
    }

    #[test]
    fn diff_test_7() {
        let ts = TestSpace::new().allow_cleanup(false);
        let mut original_file = ts.create_text_file();
        original_file.append_line("A");
        original_file.append_line("B");
        original_file.append_line("C");
        original_file.append_line("D");
        original_file.append_line("E");
        original_file.append_line("A");
        original_file.append_line("B");
        original_file.append_line("C");
        let mut new_file = original_file.create_copy();

        new_file.move_line(1, 7); // ACDEABCB
                                  // WorkingDirectory::file_patch()
        WorkingDirectory::file_patch(original_file.get_path(), new_file.get_path());
    }

    #[test]
    fn create_metadata_list_test() {
        let mut ts = TestSpace::new();
        let file_list = ts.create_random_files(2, 4096);
        // TODO: Finish test
        let metadata = WorkingDirectory::create_metadata_list(file_list)
            .expect("Failed to create metadata list");
        // println!("Data is: {:?}", metadata);
        for file in metadata {
            assert_eq!(file.filesize(), 4096);
        }
    }

    #[test]
    fn index_directory_test() {
        let mut ts = TestSpace::new();
        let mut ts2 = ts.create_child();
        ts.create_random_files(5, 4096);
        let path_to_repository = ts2.get_path();
        let path_to_working = ts.get_path();
        let ti = WorkingDirectory::new(path_to_working);
        let mut files_found = HashMap::new();
        let mut directories = Vec::new();
        WorkingDirectory::index_directory(path_to_working, &mut files_found, &mut directories)
            .expect("Failed to index directory");
        println!("Files found");
        assert_eq!(files_found.len(), 5);
        for data in files_found {
            assert_eq!(data.1.file_size, 4096);
        }
    }

    #[test]
    fn index_repository_test() {
        let mut ts = TestSpace::new();
        let mut ts2 = ts.create_child();
        // Create fake repository files
        ts2.create_random_files(5, 4096);
        let mut ts3 = ts.create_child();
        ts3.create_random_files(4, 4096);
        ts.create_random_files(3, 4096);
        let path_to_repository = ts2.get_path();
        let path_to_working = ts.get_path();
        let ti = WorkingDirectory::new(path_to_working);
        let files = ti.index_repository().expect("Failed to index repository");
        println!("Files found");
        assert_eq!(files.len(), 12);
        for data in files {
            assert_eq!(data.1.file_size, 4096);
        }
    }

    #[test]
    fn get_changed_files_test() {
        use rand::prelude::*;
        use std::path;
        let mut rng = thread_rng();
        let mut ts = TestSpace::new();
        let original_files = ts.create_random_files(6, 4096);
        let path_to_working = ts.get_path();
        let ti = WorkingDirectory::new(path_to_working);
        let working_state = WorkingDirectory::create_metadata_list(original_files.clone())
            .expect("Failed to process original files");
        // Change some of the files
        // Generate a list of files to change from the list of files
        let mut files_to_change: Vec<&path::Path> = original_files
            .choose_multiple(&mut rng, 3)
            .map(|x| x.as_path())
            .collect();
        // Change the files in the new list
        for file_to_change in &files_to_change {
            TestSpaceFile::from(*file_to_change).write_random_bytes(2048);
        }
        let mut result = ti.get_changed_files(working_state.as_slice());
        // Check that the returned files match the ones we changed
        println!("List of files: {:?}", original_files.as_slice());
        println!("List of files changed: {:?}", files_to_change.as_slice());
        println!("Detected files that were changed: {:?}", result.as_slice());
        // Sort both of the lists as the order is not guarenteed to be the same
        result.sort();
        files_to_change.sort();
        assert_eq!(result, files_to_change);
    }

    #[test]
    fn get_changed_files_since_snapshot_test() {
        unimplemented!("Test not done");
        // Take a snapshot
        // Change some files
        // Check what changed with what was changed
    }
}
