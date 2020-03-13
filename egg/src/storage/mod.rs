// Contains types that persist data to disk
pub(crate) mod storable;        // Provides implementations for all the types that need to be stored in the repository
pub(crate) mod repository;      // Helper methods for storing and retrieving metadata from the repository
pub(crate) mod data;            // Reads and writes user data to the repository
pub(crate) mod stream;          // Provides extensions to Read Write traits to allow easier reading and writing a basic egg data types

// Re-export most of these modules from here
pub(crate) use data::LocalFileStorage as LocalFileStorage;
pub(crate) use repository::RepositoryStorage as RepositoryStorage;
pub(crate) use storable::Storable as Storable;