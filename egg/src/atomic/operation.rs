use super::FileOperation;
use crate::error::{Error, UnderlyingError};
use std::path;

impl FileOperation {
    pub fn get_relative_path<'a>(&self, relative_to_path: &'a path::Path) -> Result<&'a path::Path, Error> {
        let file_path = match self {
            FileOperation::Create(path) => path,
            FileOperation::Replace(path) => path,
            FileOperation::Store(path) => path,
        };
        let relative_path = relative_to_path.strip_prefix(file_path)
            .map_err(|err| Error::invalid_parameter(Some(UnderlyingError::from(err))))?;
        Ok(relative_path)
    }

    // TODO: This isn't enough, this must return the path relative to another path, the other path being the working directory
    pub fn get_filename(&self) -> Result<&str, Error> {
        match self {
            FileOperation::Create(path) => FileOperation::get_str(path).map_err(|error| error.add_generic_message("While creating a file")),
            FileOperation::Replace(path) => FileOperation::get_str(path).map_err(|err| err.add_generic_message("While replacing a file")),
            FileOperation::Store(path) => FileOperation::get_str(path).map_err(|err| err.add_generic_message("While storing a file")),
        }
    }

    fn get_str(path: &path::Path) -> Result<&str, Error> {
        let file_name = path
                .file_name()
                    .ok_or(Error::invalid_parameter(None).add_generic_message("A path used for an atomic operation did not have a file name"))?
                .to_str()
                    .ok_or(Error::invalid_parameter(None).add_generic_message("A path used for an atomic operation was not a valid UTF8 string"))?;
        Ok(file_name)
    }
}