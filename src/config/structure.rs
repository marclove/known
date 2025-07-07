//! Defines the `Config` struct and its implementation.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Configuration structure that holds the list of watched directories
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Config {
    /// List of directories being watched for rules synchronization
    pub watched_directories: HashSet<PathBuf>,
}

impl Config {
    /// Creates a new empty configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a directory to the watched directories list
    ///
    /// # Arguments
    ///
    /// * `dir_path` - Path to the directory to add
    ///
    /// # Returns
    ///
    /// Returns `true` if the directory was added (wasn't already present),
    /// `false` if it was already in the list
    pub fn add_directory<P: AsRef<Path>>(&mut self, dir_path: P) -> bool {
        let canonical_path = match dir_path.as_ref().canonicalize() {
            Ok(path) => path,
            Err(_) => dir_path.as_ref().to_path_buf(),
        };
        self.watched_directories.insert(canonical_path)
    }

    /// Removes a directory from the watched directories list
    ///
    /// # Arguments
    ///
    /// * `dir_path` - Path to the directory to remove
    ///
    /// # Returns
    ///
    /// Returns `true` if the directory was removed (was present),
    /// `false` if it wasn't in the list
    pub fn remove_directory<P: AsRef<Path>>(&mut self, dir_path: P) -> bool {
        let canonical_path = match dir_path.as_ref().canonicalize() {
            Ok(path) => path,
            Err(_) => dir_path.as_ref().to_path_buf(),
        };
        self.watched_directories.remove(&canonical_path)
    }

    /// Checks if a directory is in the watched directories list
    ///
    /// # Arguments
    ///
    /// * `dir_path` - Path to the directory to check
    pub fn contains_directory<P: AsRef<Path>>(&self, dir_path: P) -> bool {
        let canonical_path = match dir_path.as_ref().canonicalize() {
            Ok(path) => path,
            Err(_) => dir_path.as_ref().to_path_buf(),
        };
        self.watched_directories.contains(&canonical_path)
    }

    /// Gets the list of watched directories
    pub fn get_watched_directories(&self) -> &HashSet<PathBuf> {
        &self.watched_directories
    }

    /// Gets the count of watched directories
    pub fn directory_count(&self) -> usize {
        self.watched_directories.len()
    }
}
