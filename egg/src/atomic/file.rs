use super::FileOperation;
use std::fmt;
impl std::fmt::Debug for FileOperation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FileOperation::Create(file_name) => write!(f, "FileOperation::Create({:?})", file_name),
            FileOperation::Replace(file_name) => write!(f, "FileOperation::Replace({:?})", file_name),
            FileOperation::Store(file_name) => write!(f, "FileOperation::Store({:?})", file_name),
        }
    }
}