use super::AtomicLocation;
use std::path;

impl AtomicLocation {
    pub fn get_path(&self) -> &path::Path {
        path::Path::new(self.get_str())
    }
    // FIXME: This needs to be returned in two parts or as a path
    pub fn get_str(&self) -> &str {
        match self {
            AtomicLocation::Base => "atomic",
            AtomicLocation::CreateComplete => "cc",
            AtomicLocation::CreateWorking => "cw",
            AtomicLocation::ReplaceWorking => "rw",
            AtomicLocation::ReplaceComplete => "rc",
            AtomicLocation::ReplacePrevious => "rp",
            AtomicLocation::ReplaceRemove => "rr",
            AtomicLocation::StoreWorking => "sw",
            AtomicLocation::StoreComplete => "sc",
        }
    }
}
