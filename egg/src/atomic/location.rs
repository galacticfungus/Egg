use std::path;
use super::AtomicLocation;

impl AtomicLocation {
    pub fn get_path(&self) -> &path::Path {
        path::Path::new(self.get_str())
    }

    pub fn get_str(&self) -> &str {
        match self {
            AtomicLocation::Base => "atomic",
            AtomicLocation::CreateComplete => "atomic/cc",
            AtomicLocation::CreateWorking => "atomic/cw",
            AtomicLocation::ReplaceWorking => "atomic/rw",
            AtomicLocation::ReplaceComplete => "atomic/rc",
            AtomicLocation::ReplacePrevious => "atomic/rp",
            AtomicLocation::ReplaceRemove => "atomic/rr",
            AtomicLocation::StoreWorking => "atomic/sw",
            AtomicLocation::StoreComplete => "atomic/sc",
        }
    }
}