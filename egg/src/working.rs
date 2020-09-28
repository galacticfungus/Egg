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

    pub fn process_overlapping(current_length: usize) {}

    // TODO: Tortoise and Hare approach for dealing with duplicates - cycle detection

    fn process_slice(
        lines_inserted: &mut HashMap<String, usize>,
        lines_removed: &mut HashMap<String, usize>,
        new_line: usize,
        original_line: usize,
        previous_line: usize,
        original_data: Vec<String>,
        edited_data: Vec<String>,
    ) -> SliceType {
        let mut slice_length = 0;
        let total = edited_data.len().max(original_data.len()) - new_line;
        for _ in 1..total {
            eprintln!(
                "Slice Iteration: {:?}, {:?}",
                &lines_removed, &lines_inserted
            );
            // QUESTION: Ideally we use an iterator here, if we return None the iterator stops, but dealing with overlapping slices becomes a challenge

            // Returns true if the two lines match otherwise false, this includes a line not being present etc...
            let slice_continues = match (
                original_data.get(previous_line + slice_length),
                edited_data.get(new_line + slice_length),
            ) {
                (Some(previous_line), Some(new_line)) => previous_line == new_line,
                _ => false,
            };
            // If the original file has a line for that line number and that line has already been seen then return the previous line number otherwise None
            let overlapping_slice = original_data
                .get(original_line + slice_length)
                .and_then(|line| lines_inserted.get(line));
            match (slice_continues, overlapping_slice) {
                (true, Some(overlapping_slice)) => {
                    // TODO: Here we return the longest of the two slices as the matching sequence
                    // Overlapping slice contains the other previous line
                    eprintln!(
                        "Previously removed overlapping slice found at {}",
                        overlapping_slice
                    );
                    // NOTE: So overlapping slice here points to the index in the new file
                    // NOTE: and previous line points to the slice on the left
                    // So here we return the length of both sides of the slices
                    // We always want to treat the longer slice as not moving since that means that
                    // less changes to the document
                    // EABCDPLO - Initial slice is the longest
                    // PLOABCD
                    // vs
                    // FRPLOABCD
                    // ABCDPLO - Initial slice is not the longest
                    // The two slices overlap and we want to remove PLO and insert PLO - ie move PLO
                    // We return the indexes to remove and the indexes to insert
                    // The indexes involved in inserting were already added we just need to add the ones to remove
                    return SliceType::Overlapping(0, 0);
                }
                (true, None) => {
                    eprintln!(
                        "Slice continues at {} in the new document",
                        new_line + slice_length
                    );
                    // If our position in the slice is less than the current position in the new file
                    if previous_line + slice_length < new_line {
                        // Remove items that were previously considered removed since we have not yet reached a point where previous doesn't point to unprocessed lines
                        eprintln!(
                                                "Removing previous {} from removed lines since line {} in original should be part of the slice and is less than {} which is the current line position in the new document",
                                                &original_data[previous_line + slice_length],
                                                previous_line + slice_length,
                                                new_line
                                            );
                        lines_removed.remove(&original_data[previous_line + slice_length]);
                    }
                    slice_length += 1;
                }
                (false, _) => return SliceType::Simple(slice_length), // If the original slice stops as we detect another slice we dont care we deal with that on the next iteration
            }
        }
        unreachable!("If we reach the end of the file while looking for the length of a slice then we should hit the return statement in the match first")
    }

    #[cfg(test)]
    pub fn file_patch(original_file: &path::Path, new_file: &path::Path) {
        use rand::Rng;
        use std::hash::Hasher;

        // TODO: Just for testing diffing algorithms
        let mut original_data = fs::OpenOptions::new()
            .read(true)
            .open(original_file)
            .unwrap();
        let new_data = fs::OpenOptions::new().read(true).open(new_file).unwrap();
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
        let original_data: Vec<String> = original_lines
            .map(|line| {
                line.unwrap()
                // TODO: We need to trim all spaces
                // let mut hash = ahash::AHasher::new_with_keys(valid_line.len() as u64, key1);
                // hash.write(valid_line.as_bytes());
                // hash.finish()
            })
            .collect();
        let new_data: Vec<String> = new_lines
            .map(|line| {
                line.unwrap()
                // let mut hash = ahash::AHasher::new_with_keys(valid_line.len() as u64, key1);
                // hash.write(valid_line.as_bytes());
                // hash.finish()
            })
            .collect();
        println!("Original File {:?}", original_data);
        println!("New File {:?}", new_data);

        let mut lines_inserted = HashMap::new();
        let mut lines_removed = HashMap::new();
        // Lines that were thought to have been removed but were moved
        let mut lines_removed_move = HashMap::new();
        // Lines that were thought to have been inserted but were moved
        let mut lines_inserted_move = HashMap::new();
        // We only use the results for each section we process from one of the above move hash maps
        let mut new_line: usize = 0;
        let mut original_line: usize = 0;
        // TODO: Do we replace this loop with an iterator
        loop {
            // TODO: Remove this initial branch and instead compute the total iterations required as well as the additional reads required at the end for the longer file or use an iterator that returns None when the file has no more data
            if original_line < original_data.len() && new_line < new_data.len() {
                if original_data[original_line] == new_data[new_line] {
                    // Lines are the same, just move to the next set of lines
                    original_line += 1;
                    new_line += 1;
                } else {
                    // Compares the line from the original text with any previous unmatched lines from the new text and vice versa
                    // NOTE: Lines that were considered removed will be found in new_data since removed lines were present in original but not in changed
                    // NOTE: Lines that were considered inserted will be found in original since inserted lines were present in changed but not in original
                    match (
                        lines_removed.get(&new_data[new_line]),
                        lines_inserted.get(original_data[original_line].as_str()),
                    ) {
                        (Some(previously_removed), Some(previously_inserted)) => {
                            // Both lines have been seen so two slices are overlapping
                            println!("Both have been seen so guarenteed overlapping move");
                            return WorkingDirectory::process_overlapping(6);
                            // We check both slices until one of them ends, the one that ends first is the one we use
                            // Here the two overlapping slices begin at the same point
                            original_line += 1;
                            new_line += 1;
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
                            eprintln!("A line previously thought to be removed has been seen, {} has been seen at line {}, {} has not", new_data[new_line].as_str(), previous_line, original_data[original_line].as_str());
                            // We already know that the slice is at least 1 length so we start from 1
                            let mut slice_length = 1;
                            let mut index_to_check = slice_length + 1;
                            eprintln!("{} is no longer considered removed", &new_data[new_line]);
                            lines_removed.remove(new_data[new_line].as_str());
                            // TODO: Deal with zero before starting loop
                            // A slice can only be as long as the smallest file or section if we have broken the file into parts
                            let total = new_data.len().max(original_data.len()) - new_line;
                            eprintln!("Processing a slice that is potentially {} long", total);
                            // TODO: A loop here is not ideal but it can be vectorized
                            // Determine the slice length or where the slice ends
                            // TODO: By always checking for matches one element ahead we know when to stop adding items to the removed lines and thus avoid 2 loops
                            // TODO: We always check ahead by one to see if we add the opposite side to the removed lines
                            for _ in 1..total {
                                eprintln!(
                                    "Slice Iteration: {:?}, {:?}",
                                    &lines_removed, &lines_inserted
                                );
                                // QUESTION: Ideally we use an iterator here, if we return None the iterator stops, but dealing with overlapping slices becomes a challenge

                                // Returns true if the two lines match otherwise false, this includes a line not being present etc...
                                let slice_continues = match (
                                    original_data.get(previous_line + slice_length),
                                    new_data.get(new_line + slice_length),
                                ) {
                                    (Some(previous_line), Some(new_line)) => {
                                        previous_line == new_line
                                    }
                                    _ => false,
                                };
                                // If the original file has a line for that line number and that line has already been seen then return the previous line number otherwise None
                                let overlapping_slice = original_data
                                    .get(original_line + slice_length)
                                    .and_then(|line| lines_inserted.get(line));
                                // We match against the next index as well if it exists
                                // slice_continues, slice_continues + 1, overlapping_slice
                                match (slice_continues, overlapping_slice) {
                                    (true, Some(overlapping_slice)) => {
                                        // TODO: Here we return the longest of the two slices as the matching sequence
                                        // NOTE: Overlapping slice where the initial slice was right?
                                        eprintln!(
                                            "Previously removed overlapping slice found at {}",
                                            overlapping_slice
                                        );
                                        // NOTE: So overlapping slice here points to the index in the new file
                                        // NOTE: and previous line points to the slice on the left
                                    }
                                    (true, None) => {
                                        eprintln!(
                                            "Slice continues at {} in the new document",
                                            new_line + slice_length
                                        );
                                        // If our position in the slice is less than the current position in the new file
                                        if previous_line + slice_length < new_line {
                                            // Remove items that were previously considered removed since we have not yet reached a point where previous doesn't point to unprocessed lines
                                            eprintln!(
                                                "Removing previous {} from removed lines since line {} in original should be part of the slice and is less than {} which is the current line position in the new document",
                                                &original_data[previous_line + slice_length],
                                                previous_line + slice_length,
                                                new_line
                                            );
                                            lines_removed.remove(
                                                &original_data[previous_line + slice_length],
                                            );
                                        }
                                        // If next item is != then the slice will end now and the opposite value needs to be added to removed
                                        slice_length += 1;
                                    }
                                    (false, _) => break, // If the original slice stops as we detect another slice we dont care we deal with that on the next iteration
                                }
                            }
                            // Get the number of items to add - this is either the offset between previous line and current line
                            // or the length of the slice, which ever is smaller
                            let offset = new_line - previous_line;
                            let items_to_add = slice_length.min(offset);
                            eprintln!("Need to add {} items to the hashmap", items_to_add);
                            // Starting offset is always new_line + slice_length and the end offset is starting_offset - items_to_add
                            let end_of_slice = new_line + slice_length;
                            let start_of_slice = end_of_slice - items_to_add;
                            eprintln!("Start at {}, end at {}", start_of_slice, end_of_slice - 1);
                            for index in start_of_slice..end_of_slice {
                                // TODO: Doesn't take into account a shorter original file
                                match original_data.get(index) {
                                    Some(line_removed) => {
                                        eprintln!("Adding index {} to inserted", index);
                                        lines_removed.insert(line_removed.clone(), index);
                                    }
                                    None => break, // The original file has run out of lines so there is no matching items for the sequence to have missed
                                                   // TODO: Does this cause a problem when the function returns
                                }
                            }

                            new_line += slice_length;
                            original_line += slice_length;
                            eprintln!(
                                "After slice: Original Line: {}, New Line: {}",
                                original_line, new_line
                            );
                        }
                        (None, Some(previously_inserted)) => {
                            // Example of match
                            // A K
                            // B L
                            // C M
                            // K N <- Previously thought to be inserted
                            // TODO: Deal with the unseen value in new_data[new_line] by adding it to removed
                            let previous_line = *previously_inserted;
                            // We already know that the slice is at least 1 length so we start from 1
                            let mut slice_length = 1;
                            eprintln!(
                                "No longer considered removed {}",
                                &original_data[original_line]
                            );
                            lines_inserted.remove(original_data[original_line].as_str());
                            // TODO: Deal with zero before starting loop
                            // A slice can only be as long as the smallest file or section if we have broken the file into parts
                            let total = new_data.len().max(original_data.len()) - original_line;
                            // TODO: A loop here is not ideal but it can be vectorized
                            // Determine the slice length or where the slice ends
                            for _ in 1..total {
                                // QUESTION: Ideally we use an iterator here, if we return None the iterator stops, but dealing with overlapping slices becomes a challenge

                                // Returns true if the two lines match otherwise false, this includes a line not being present etc...
                                let slice_continues = match (
                                    original_data.get(original_line + slice_length),
                                    new_data.get(previous_line + slice_length),
                                ) {
                                    (Some(previous_line), Some(new_line)) => {
                                        previous_line == new_line
                                    }
                                    _ => false,
                                };
                                // If the new file has a line for that line number and that line has already been seen then return the previous line number otherwise None
                                let overlapping_slice = new_data
                                    .get(new_line + slice_length)
                                    .and_then(|line| lines_removed.get(line));
                                match (slice_continues, overlapping_slice) {
                                    (true, Some(overlapping_slice)) => {
                                        // TODO: Here we return the longest of the two slices as the matching sequence
                                        eprintln!(
                                            "Overlapping slice found at {}",
                                            overlapping_slice
                                        );
                                    }
                                    (true, None) => {
                                        eprintln!(
                                            "Slice continues at {} in the original document",
                                            original_line + slice_length
                                        );
                                        // If our position in the slice is less than the current position in the new file
                                        if previous_line + slice_length < original_line {
                                            // Remove items that were previously considered removed since we have not yet reached a point where previous doesn't point to unprocessed lines
                                            eprintln!(
                                                "Removing previous {} from inserted lines since line {} in new should be part of the slice and is less than {} which is the current line position in the original document",
                                                &new_data[previous_line + slice_length],
                                                previous_line + slice_length,
                                                original_line
                                            );
                                            lines_inserted
                                                .remove(&new_data[previous_line + slice_length]);
                                        }
                                        slice_length += 1;
                                    }
                                    (false, _) => break, // If the original slice stops as we detect another slice we dont care we deal with that on the next iteration
                                }
                            }
                            // Get the number of items to add - this is either the offset between previous line and current line
                            // or the length of the slice, which ever is smaller
                            let offset = original_line - previous_line;
                            let items_to_add = slice_length.min(offset);
                            eprintln!("Need to add {} items to the hashmap", items_to_add);
                            // Starting offset is always new_line + slice_length and the end offset is starting_offset - items_to_add
                            let end_of_slice = original_line + slice_length;
                            let start_of_slice = end_of_slice - items_to_add;
                            eprintln!("Start at {}, end at {}", start_of_slice, end_of_slice - 1);
                            for index in start_of_slice..end_of_slice {
                                match new_data.get(index) {
                                    Some(line_inserted) => {
                                        eprintln!("Adding index {} to inserted", index);
                                        lines_inserted.insert(line_inserted.clone(), index);
                                    }
                                    None => break, // The new file ran out of lines so the slice can't be next to inserted lines
                                }
                            }

                            new_line += slice_length;
                            original_line += slice_length;
                            eprintln!(
                                "After slice: Original Line: {}, New Line: {}",
                                original_line, new_line
                            );
                        }
                        (None, None) => {
                            // Neither of these lines have been seen before
                            // So we they are prospective removed and inserted lines
                            println!(
                                "We have not seen {} at line {} or {} at line {} before",
                                &original_data[original_line],
                                original_line,
                                &new_data[new_line],
                                new_line
                            );
                            // This does not mean that these lines are not duplicates
                            // Check for a new_data[new_line] in lines_inserted and if so edit the entry with the an additional line
                            lines_inserted.insert(new_data[new_line].clone(), new_line);
                            lines_removed
                                .insert(original_data[original_line].clone(), original_line);
                            new_line += 1;
                            original_line += 1;
                        }
                    }
                    println!("----------------------------------------------------------------------------------------------------");
                }
            } else if original_line < original_data.len() {
                // We have run out of lines in the new file
                // Check original line to see if it is listed in the inserted lines = becomes moved
                if lines_inserted.contains_key(&original_data[original_line]) {
                    // This was a move and not a insert
                    // TODO: We can still have slices
                    let (previous_insert, previous_insert_line) = lines_inserted
                        .remove_entry(&original_data[original_line])
                        .unwrap();
                    println!("Line {} was moved not inserted", original_line);
                    lines_inserted_move
                        .insert(previous_insert, (original_line, previous_insert_line));
                } else {
                    lines_removed.insert(original_data[original_line].clone(), original_line);
                }
                original_line += 1;
            } else if new_line < new_data.len() {
                // We have run out of lines in the old file
                // Check new line to see if it is listed in the removed lines = becomes moved and check the index difference for alignment
                if lines_removed.contains_key(&new_data[new_line]) {
                    // TODO: We can still have slices
                    // We need to check for overlapping slices, otherwise we ignore them - but they can never overlap
                    // This was a move and not a removal
                    let (previous_remove, previous_remove_line) =
                        lines_removed.remove_entry(&new_data[new_line]).unwrap();
                    println!("Line {} was moved not removed", previous_remove_line);
                    lines_removed_move.insert(previous_remove, (previous_remove_line, new_line));
                } else {
                    lines_inserted.insert(new_data[new_line].clone(), new_line);
                }
                new_line += 1;
            // Add new to inserted
            } else {
                // Nothing left to scan
                break;
            }
        }
        println!("Inserted {:?}", lines_inserted);
        println!("Lines removed {:?}", lines_removed);
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
        WorkingDirectory::file_patch(original_file.get_path(), changed_file.get_path());
    }

    #[test]
    fn basic_previously_removed_test2() {
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
                                       // JKABC
                                       // ABCDE
        WorkingDirectory::file_patch(original_file.get_path(), changed_file.get_path());
    }

    #[test]
    fn length_of_one_sequence_removed_test() {
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
        WorkingDirectory::file_patch(original_file.get_path(), changed_file.get_path());
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
        WorkingDirectory::file_patch(original_file.get_path(), changed_file.get_path());
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
        WorkingDirectory::file_patch(original_file.get_path(), changed_file.get_path());
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
        WorkingDirectory::file_patch(original_file.get_path(), changed_file.get_path());
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
        WorkingDirectory::file_patch(original_file.get_path(), changed_file.get_path());
    }

    #[test]
    fn inserted_uneven_lengths_test() {
        let ts = TestSpace::new();
        let mut original_file = ts.create_text_file();
        original_file.append_line("D"); // 0
        original_file.append_line("E"); // 1
        original_file.append_line("A"); // 0
        original_file.append_line("B"); // 1
        original_file.append_line("C"); // 2
        let mut changed_file = ts.create_text_file();
        changed_file.append_line("A"); // 2
        changed_file.append_line("B"); // 3
        changed_file.append_line("C"); // 4
                                       // ABC
                                       // DEABC
        WorkingDirectory::file_patch(original_file.get_path(), changed_file.get_path());
    }

    #[test]
    fn basic_slice_test() {
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
    fn overlapping_sequence_test() {
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
        changed_file.append_line("G"); // 0
        changed_file.append_line("H"); // 1
        changed_file.append_line("I"); // 2
        changed_file.append_line("A"); // 3
        changed_file.append_line("B"); // 4
        changed_file.append_line("C"); // 5
        changed_file.append_line("D"); // 6
                                       // JKABC
                                       // ABCDE
        WorkingDirectory::file_patch(original_file.get_path(), changed_file.get_path());
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
