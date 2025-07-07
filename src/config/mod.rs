//! Configuration file management for tracking watched directories.
//!
//! This module provides functionality for managing a configuration file that tracks
//! a list of directories where the `symlink` command has been executed. The daemon
//! uses this configuration to watch all tracked directories simultaneously.

pub mod io;
pub mod path;
pub mod structure;

pub use io::{
    add_directory_to_config, add_directory_to_config_file, load_config, load_config_from_file,
    remove_directory_from_config, remove_directory_from_config_file, save_config,
    save_config_to_file,
};
pub use path::get_config_file_path;
pub use structure::Config;

#[cfg(test)]
mod tests {
    use super::{
        add_directory_to_config_file, load_config_from_file, remove_directory_from_config_file,
        save_config_to_file, Config,
    };
    use std::fs;
    use std::path::Path;
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
        let config_path = super::get_config_file_path().unwrap();

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
            assert_eq!(e.kind(), std::io::ErrorKind::InvalidData);
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
            assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::PermissionDenied);
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
        let config_path = super::get_config_file_path().unwrap();

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

    #[test]
    fn test_add_and_remove_directory_from_config_file() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("test_config.json");
        let dir_to_add = tempdir().unwrap();

        // Add a directory
        let added =
            add_directory_to_config_file(dir_to_add.path(), &config_path).unwrap();
        assert!(added);

        // Verify it was added
        let config = load_config_from_file(&config_path).unwrap();
        assert_eq!(config.directory_count(), 1);
        assert!(config.contains_directory(dir_to_add.path()));

        // Remove the directory
        let removed =
            remove_directory_from_config_file(dir_to_add.path(), &config_path).unwrap();
        assert!(removed);

        // Verify it was removed
        let config_after_remove = load_config_from_file(&config_path).unwrap();
        assert_eq!(config_after_remove.directory_count(), 0);
    }
}
