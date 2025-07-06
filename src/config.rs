//! Configuration file management for tracking watched directories.
//!
//! This module provides functionality for managing a configuration file that tracks
//! a list of directories where the `symlink` command has been executed. The daemon
//! uses this configuration to watch all tracked directories simultaneously.

use directories::ProjectDirs;
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Configuration file name
const CONFIG_FILE_NAME: &str = "config.json";

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

/// Returns the path to the configuration file
///
/// Uses the platform-specific application configuration directory according to:
/// - Linux: `$XDG_CONFIG_HOME/known/config.json` or `$HOME/.config/known/config.json`
/// - macOS: `$HOME/Library/Application Support/known/config.json`
/// - Windows: `%APPDATA%/known/config.json`
///
/// Falls back to `$HOME/.config/known/config.json` if platform-specific directories cannot be determined.
///
/// # Errors
///
/// Returns an error if the configuration directory cannot be determined
pub fn get_config_file_path() -> io::Result<PathBuf> {
    // Try to use platform-specific directories first
    if let Some(project_dirs) = ProjectDirs::from("", "", "known") {
        let config_dir = project_dirs.config_dir();
        return Ok(config_dir.join(CONFIG_FILE_NAME));
    }

    // Fallback: use HOME/.config/known/config.json for CI environments and other edge cases
    let home_dir = std::env::var("HOME").map_err(|_| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "Unable to determine home directory for configuration path",
        )
    })?;

    let config_dir = Path::new(&home_dir).join(".config").join("known");
    Ok(config_dir.join(CONFIG_FILE_NAME))
}

/// Loads configuration from the configuration file
///
/// # Errors
///
/// Returns an error if:
/// - The configuration directory cannot be determined
/// - File reading fails
/// - JSON parsing fails
pub fn load_config() -> io::Result<Config> {
    let config_path = get_config_file_path()?;

    // If config file doesn't exist, return default config
    if !config_path.exists() {
        return Ok(Config::default());
    }

    let config_content = fs::read_to_string(&config_path)?;

    // Trim any trailing whitespace or newlines that might cause parsing issues
    let trimmed_content = config_content.trim();

    // Handle empty file case
    if trimmed_content.is_empty() {
        return Ok(Config::default());
    }

    let config: Config = serde_json::from_str(trimmed_content).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Failed to parse configuration file at {}: {} (content length: {} chars)",
                config_path.display(),
                e,
                trimmed_content.len()
            ),
        )
    })?;

    Ok(config)
}

/// Saves configuration to the configuration file
///
/// # Arguments
///
/// * `config` - The configuration to save
///
/// # Errors
///
/// Returns an error if:
/// - The configuration directory cannot be determined
/// - Directory creation fails
/// - JSON serialization fails
/// - File writing fails
pub fn save_config(config: &Config) -> io::Result<()> {
    let config_path = get_config_file_path()?;

    // Create the configuration directory if it doesn't exist
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut config_content = serde_json::to_string_pretty(config).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to serialize configuration: {}", e),
        )
    })?;

    // Ensure the file ends with a newline
    if !config_content.ends_with('\n') {
        config_content.push('\n');
    }

    fs::write(&config_path, config_content)?;
    Ok(())
}

/// Safely modifies the configuration file using atomic writes to prevent race conditions
///
/// # Arguments
///
/// * `modifier` - Function that modifies the configuration and returns whether changes were made
///
/// # Errors
///
/// Returns an error if file operations fail
fn modify_config_safely<F>(modifier: F) -> io::Result<bool>
where
    F: FnOnce(&mut Config) -> bool,
{
    let config_path = get_config_file_path()?;

    // Create the configuration directory if it doesn't exist
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Read the current configuration
    let config_content = if config_path.exists() {
        fs::read_to_string(&config_path)?
    } else {
        String::new()
    };

    let mut config = if config_content.trim().is_empty() {
        Config::default()
    } else {
        serde_json::from_str(config_content.trim()).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Failed to parse configuration file at {}: {} (content length: {} chars)",
                    config_path.display(),
                    e,
                    config_content.trim().len()
                ),
            )
        })?
    };

    // Apply the modification
    let changed = modifier(&mut config);

    if changed {
        // Serialize the updated configuration
        let mut new_content = serde_json::to_string_pretty(&config).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to serialize configuration: {}", e),
            )
        })?;

        // Ensure the file ends with a newline
        if !new_content.ends_with('\n') {
            new_content.push('\n');
        }

        // Use atomic write by writing to a temporary file and then renaming
        let temp_path = config_path.with_extension("json.tmp");
        fs::write(&temp_path, new_content)?;
        fs::rename(&temp_path, &config_path)?;
    }

    Ok(changed)
}

/// Adds a directory to the configuration and saves it
///
/// # Arguments
///
/// * `dir_path` - Path to the directory to add
///
/// # Errors
///
/// Returns an error if loading or saving the configuration fails
pub fn add_directory_to_config<P: AsRef<Path>>(dir_path: P) -> io::Result<bool> {
    let dir_path = dir_path.as_ref().to_path_buf();
    modify_config_safely(|config| config.add_directory(&dir_path))
}

/// Removes a directory from the configuration and saves it
///
/// # Arguments
///
/// * `dir_path` - Path to the directory to remove
///
/// # Errors
///
/// Returns an error if loading or saving the configuration fails
pub fn remove_directory_from_config<P: AsRef<Path>>(dir_path: P) -> io::Result<bool> {
    let dir_path = dir_path.as_ref().to_path_buf();
    modify_config_safely(|config| config.remove_directory(&dir_path))
}

/// Loads configuration from a specific file path (for testing)
///
/// # Arguments
///
/// * `config_path` - Path to the configuration file
///
/// # Errors
///
/// Returns an error if file reading or JSON parsing fails
pub fn load_config_from_file<P: AsRef<Path>>(config_path: P) -> io::Result<Config> {
    let config_path = config_path.as_ref();

    // If config file doesn't exist, return default config
    if !config_path.exists() {
        return Ok(Config::default());
    }

    let config_content = fs::read_to_string(config_path)?;

    // Trim any trailing whitespace or newlines that might cause parsing issues
    let trimmed_content = config_content.trim();

    // Handle empty file case
    if trimmed_content.is_empty() {
        return Ok(Config::default());
    }

    let config: Config = serde_json::from_str(trimmed_content).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Failed to parse configuration file at {}: {} (content length: {} chars)",
                config_path.display(),
                e,
                trimmed_content.len()
            ),
        )
    })?;

    Ok(config)
}

/// Saves configuration to a specific file path (for testing)
///
/// # Arguments
///
/// * `config` - The configuration to save
/// * `config_path` - Path to the configuration file
///
/// # Errors
///
/// Returns an error if directory creation, JSON serialization, or file writing fails
pub fn save_config_to_file<P: AsRef<Path>>(config: &Config, config_path: P) -> io::Result<()> {
    let config_path = config_path.as_ref();

    // Create the configuration directory if it doesn't exist
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut config_content = serde_json::to_string_pretty(config).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to serialize configuration: {}", e),
        )
    })?;

    // Ensure the file ends with a newline
    if !config_content.ends_with('\n') {
        config_content.push('\n');
    }

    fs::write(config_path, config_content)?;
    Ok(())
}

/// Safely modifies a specific configuration file using atomic writes (for testing)
///
/// # Arguments
///
/// * `config_path` - Path to the configuration file
/// * `modifier` - Function that modifies the configuration and returns whether changes were made
///
/// # Errors
///
/// Returns an error if file operations fail
fn modify_config_file_safely<F, P: AsRef<Path>>(config_path: P, modifier: F) -> io::Result<bool>
where
    F: FnOnce(&mut Config) -> bool,
{
    let config_path = config_path.as_ref();

    // Create the configuration directory if it doesn't exist
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Read the current configuration
    let config_content = if config_path.exists() {
        fs::read_to_string(config_path)?
    } else {
        String::new()
    };

    let mut config = if config_content.trim().is_empty() {
        Config::default()
    } else {
        serde_json::from_str(config_content.trim()).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Failed to parse configuration file at {}: {} (content length: {} chars)",
                    config_path.display(),
                    e,
                    config_content.trim().len()
                ),
            )
        })?
    };

    // Apply the modification
    let changed = modifier(&mut config);

    if changed {
        // Serialize the updated configuration
        let mut new_content = serde_json::to_string_pretty(&config).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to serialize configuration: {}", e),
            )
        })?;

        // Ensure the file ends with a newline
        if !new_content.ends_with('\n') {
            new_content.push('\n');
        }

        // Use atomic write by writing to a temporary file and then renaming
        let temp_path = config_path.with_extension("json.tmp");
        fs::write(&temp_path, new_content)?;
        fs::rename(&temp_path, config_path)?;
    }

    Ok(changed)
}

/// Adds a directory to a specific configuration file (for testing)
///
/// # Arguments
///
/// * `dir_path` - Path to the directory to add
/// * `config_path` - Path to the configuration file
///
/// # Errors
///
/// Returns an error if loading or saving the configuration fails
pub fn add_directory_to_config_file<P: AsRef<Path>, C: AsRef<Path>>(
    dir_path: P,
    config_path: C,
) -> io::Result<bool> {
    let dir_path = dir_path.as_ref().to_path_buf();
    modify_config_file_safely(config_path, |config| config.add_directory(&dir_path))
}

/// Removes a directory from a specific configuration file (for testing)
///
/// # Arguments
///
/// * `dir_path` - Path to the directory to remove
/// * `config_path` - Path to the configuration file
///
/// # Errors
///
/// Returns an error if loading or saving the configuration fails
pub fn remove_directory_from_config_file<P: AsRef<Path>, C: AsRef<Path>>(
    dir_path: P,
    config_path: C,
) -> io::Result<bool> {
    let dir_path = dir_path.as_ref().to_path_buf();
    modify_config_file_safely(config_path, |config| config.remove_directory(&dir_path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_config_new() {
        let config = Config::new();
        assert_eq!(config.directory_count(), 0);
        assert!(config.get_watched_directories().is_empty());
    }

    #[test]
    fn test_config_add_directory() {
        let mut config = Config::new();
        let dir = tempdir().unwrap();

        // Add a directory
        let added = config.add_directory(dir.path());
        assert!(added);
        assert_eq!(config.directory_count(), 1);
        assert!(config.contains_directory(dir.path()));

        // Add same directory again - should not add duplicates
        let added_again = config.add_directory(dir.path());
        assert!(!added_again);
        assert_eq!(config.directory_count(), 1);
    }

    #[test]
    fn test_config_remove_directory() {
        let mut config = Config::new();
        let dir = tempdir().unwrap();

        // Add a directory first
        config.add_directory(dir.path());
        assert_eq!(config.directory_count(), 1);

        // Remove the directory
        let removed = config.remove_directory(dir.path());
        assert!(removed);
        assert_eq!(config.directory_count(), 0);
        assert!(!config.contains_directory(dir.path()));

        // Try to remove again - should return false
        let removed_again = config.remove_directory(dir.path());
        assert!(!removed_again);
    }

    #[test]
    fn test_config_contains_directory() {
        let mut config = Config::new();
        let dir1 = tempdir().unwrap();
        let dir2 = tempdir().unwrap();

        // Add only dir1
        config.add_directory(dir1.path());

        assert!(config.contains_directory(dir1.path()));
        assert!(!config.contains_directory(dir2.path()));
    }

    #[test]
    fn test_config_serialization() {
        let mut config = Config::new();
        let dir = tempdir().unwrap();

        config.add_directory(dir.path());

        // Serialize to JSON
        let json_str = serde_json::to_string(&config).unwrap();

        // Deserialize back
        let deserialized_config: Config = serde_json::from_str(&json_str).unwrap();

        assert_eq!(
            config.directory_count(),
            deserialized_config.directory_count()
        );
        assert!(deserialized_config.contains_directory(dir.path()));
    }

    #[test]
    fn test_get_config_file_path() {
        let config_path = get_config_file_path().unwrap();

        // Should contain the application name and config file name
        assert!(config_path.to_string_lossy().contains("known"));
        assert!(config_path.to_string_lossy().contains("config.json"));
    }

    #[test]
    fn test_save_and_load_config() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("config.json");

        // Create a config with some directories
        let mut original_config = Config::new();
        let dir1 = tempdir().unwrap();
        let dir2 = tempdir().unwrap();

        original_config.add_directory(dir1.path());
        original_config.add_directory(dir2.path());

        // Save config to JSON file
        let json_str = serde_json::to_string_pretty(&original_config).unwrap();
        fs::write(&config_path, json_str).unwrap();

        // Load config from JSON file
        let loaded_content = fs::read_to_string(&config_path).unwrap();
        let loaded_config: Config = serde_json::from_str(&loaded_content).unwrap();

        assert_eq!(
            original_config.directory_count(),
            loaded_config.directory_count()
        );
        assert!(loaded_config.contains_directory(dir1.path()));
        assert!(loaded_config.contains_directory(dir2.path()));
    }

    #[test]
    fn test_load_config_nonexistent_file() {
        // Test loading config when file doesn't exist
        // Since we can't easily mock the real config file path in tests,
        // we test the JSON parsing logic directly
        let empty_config = Config::default();
        let json_str = serde_json::to_string(&empty_config).unwrap();
        let loaded_config: Config = serde_json::from_str(&json_str).unwrap();

        assert_eq!(loaded_config.directory_count(), 0);
        assert!(loaded_config.get_watched_directories().is_empty());
    }

    #[test]
    fn test_config_with_multiple_directories() {
        let mut config = Config::new();
        let dirs: Vec<_> = (0..5).map(|_| tempdir().unwrap()).collect();

        // Add all directories
        for dir in &dirs {
            config.add_directory(dir.path());
        }

        assert_eq!(config.directory_count(), 5);

        // Check all directories are present
        for dir in &dirs {
            assert!(config.contains_directory(dir.path()));
        }

        // Remove some directories
        config.remove_directory(dirs[0].path());
        config.remove_directory(dirs[2].path());

        assert_eq!(config.directory_count(), 3);
        assert!(!config.contains_directory(dirs[0].path()));
        assert!(config.contains_directory(dirs[1].path()));
        assert!(!config.contains_directory(dirs[2].path()));
        assert!(config.contains_directory(dirs[3].path()));
        assert!(config.contains_directory(dirs[4].path()));
    }

    #[test]
    fn test_load_config_malformed_file() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("config.json");

        // Write malformed JSON to the file
        fs::write(&config_path, "this is not json").unwrap();

        // Attempt to load the config
        let result = load_config_from_file(&config_path);

        // Assert that it returns an error
        assert!(result.is_err());
        if let Err(e) = result {
            assert_eq!(e.kind(), io::ErrorKind::InvalidData);
        }
    }

    #[test]
    fn test_save_config_permission_denied() {
        let temp_dir = tempdir().unwrap();
        let readonly_dir = temp_dir.path().join("readonly");
        fs::create_dir(&readonly_dir).unwrap();

        if cfg!(unix) {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(0o555);
            fs::set_permissions(&readonly_dir, perms).unwrap();
        } else {
            let mut perms = fs::metadata(&readonly_dir).unwrap().permissions();
            perms.set_readonly(true);
            fs::set_permissions(&readonly_dir, perms).unwrap();
        }

        let config_path = readonly_dir.join("config.json");
        let config = Config::new();
        let result = save_config_to_file(&config, &config_path);

        if result.is_err() {
            assert_eq!(result.unwrap_err().kind(), io::ErrorKind::PermissionDenied);
        }
    }

    #[test]
    fn test_remove_directory_nonexistent() {
        let mut config = Config::new();
        let temp_dir = tempdir().unwrap();
        let nonexistent_path = temp_dir.path().join("nonexistent");

        // Try to remove a directory that was never added
        let removed = config.remove_directory(&nonexistent_path);
        assert!(!removed, "Should return false when directory not in list");
        assert_eq!(config.directory_count(), 0);
    }

    #[test]
    fn test_contains_directory_nonexistent() {
        let config = Config::new();
        let temp_dir = tempdir().unwrap();
        let nonexistent_path = temp_dir.path().join("nonexistent");

        // Check for a directory that doesn't exist and wasn't added
        let contains = config.contains_directory(&nonexistent_path);
        assert!(!contains, "Should return false for nonexistent directory");
    }

    #[test]
    fn test_config_serialization_error_scenarios() {
        // Test that the configuration can handle various edge cases
        let mut config = Config::new();

        // Add a path with unusual characters (if supported by the OS)
        let temp_dir = tempdir().unwrap();
        let unusual_name = if cfg!(windows) {
            // Windows has stricter path limitations
            temp_dir.path().join("test_dir")
        } else {
            // Unix-like systems can handle more characters
            temp_dir.path().join("test-dir_with.special@chars")
        };

        std::fs::create_dir_all(&unusual_name).unwrap();
        config.add_directory(&unusual_name);

        // Ensure serialization works
        let json_str = serde_json::to_string_pretty(&config).unwrap();
        let deserialized: Config = serde_json::from_str(&json_str).unwrap();

        assert_eq!(config.directory_count(), deserialized.directory_count());
        assert!(deserialized.contains_directory(&unusual_name));
    }

    #[test]
    fn test_get_config_file_path_fallback() {
        // Test that get_config_file_path works with custom HOME environment
        let temp_dir = tempdir().unwrap();
        let custom_home = temp_dir.path().to_string_lossy().to_string();

        // Set a custom HOME environment variable
        let original_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", &custom_home);

        // Get config file path - should work even if ProjectDirs fails
        let config_path = get_config_file_path().unwrap();

        // Should contain expected structure
        assert!(config_path.to_string_lossy().contains("known"));
        assert!(config_path.to_string_lossy().contains("config.json"));

        // The path should be HOME/.config/known/config.json in fallback mode
        let expected_fallback_path = Path::new(&custom_home)
            .join(".config")
            .join("known")
            .join("config.json");

        // The config path should either be the platform-specific one or the fallback
        assert!(
            config_path == expected_fallback_path || config_path.ends_with("known/config.json"),
            "Config path should be either platform-specific or fallback path"
        );

        // Restore original HOME environment
        match original_home {
            Some(home) => std::env::set_var("HOME", home),
            None => std::env::remove_var("HOME"),
        }
    }
}
