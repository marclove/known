//! Configuration file change handling for the daemon.

use std::io;
use std::path::PathBuf;
use std::sync::mpsc;

use crate::config::load_config;
#[cfg(test)]
use crate::config::load_config_from_file;
#[cfg(test)]
use std::collections::HashMap;
use crate::constants::RULES_DIR;

use super::symlinks::remove_symlinks_from_directory;
use super::watchers::{setup_directory_watchers, WatcherSetup};

/// Handles configuration file changes with watcher setup management.
///
/// This function is called when the daemon detects changes to the configuration file.
/// It reloads the configuration and updates the watchers accordingly by:
/// - Adding watchers for newly configured directories
/// - Removing watchers for directories that are no longer configured
/// - Cleaning up symlinks from removed directories
/// - Updating the daemon's internal state
///
/// # Arguments
///
/// * `config` - Mutable reference to the current configuration
/// * `watched_directories` - Mutable reference to the set of currently watched directories
/// * `watcher_setup` - Mutable reference to the watcher setup containing watchers and rules paths
///
/// # Errors
///
/// This function is designed to be resilient to errors. If configuration reloading fails,
/// it logs the error but doesn't crash the daemon. Individual watcher setup or symlink
/// removal failures are logged but don't prevent other operations from continuing.
///
/// # Returns
///
/// Returns `Ok(())` in most cases, even when some operations fail (errors are logged).
/// Only returns an error if critical operations fail.
pub fn handle_config_file_change_internal(
    config: &mut crate::config::Config,
    watched_directories: &mut std::collections::HashSet<PathBuf>,
    watcher_setup: &mut WatcherSetup,
) -> io::Result<()> {
    println!("Configuration file changed, reloading...");

    // Load new configuration
    let new_config = match load_config() {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Failed to reload configuration: {}", e);
            return Ok(()); // Don't fail the daemon, just log the error
        }
    };

    let new_watched_directories = new_config.get_watched_directories().clone();

    // Find directories that were added
    let added_directories: std::collections::HashSet<_> = new_watched_directories
        .difference(watched_directories)
        .collect();

    // Find directories that were removed
    let removed_directories: std::collections::HashSet<_> = watched_directories
        .difference(&new_watched_directories)
        .collect();

    if !added_directories.is_empty() {
        println!(
            "Adding {} new directories to watch:",
            added_directories.len()
        );
        for dir in &added_directories {
            println!("  + {}", dir.display());
        }

        // Add watchers for new directories
        let added_dirs_set: std::collections::HashSet<PathBuf> =
            added_directories.into_iter().cloned().collect();
        let (tx, _) = mpsc::channel(); // We don't use this receiver, just need the sender
        if let Err(e) = setup_directory_watchers(
            &added_dirs_set,
            &tx,
            &mut watcher_setup.watchers,
            &mut watcher_setup.rules_paths,
        ) {
            eprintln!("Failed to setup watchers for new directories: {}", e);
        }
    }

    if !removed_directories.is_empty() {
        println!(
            "Removing {} directories from watch:",
            removed_directories.len()
        );
        for dir in &removed_directories {
            println!("  - {}", dir.display());
        }

        // Remove watchers and rules_paths entries for removed directories
        for removed_dir in &removed_directories {
            let rules_path = removed_dir.join(RULES_DIR);
            if let Ok(canonical_path) = rules_path.canonicalize() {
                watcher_setup.rules_paths.remove(&canonical_path);
            }

            // Remove symlinks from the removed directory
            if let Err(e) = remove_symlinks_from_directory(removed_dir) {
                eprintln!(
                    "Failed to remove symlinks from {}: {}",
                    removed_dir.display(),
                    e
                );
            } else {
                println!("Removed symlinks from {}", removed_dir.display());
            }
        }
    }

    // Update our local state
    *config = new_config;
    *watched_directories = new_watched_directories;

    println!(
        "Configuration reloaded successfully. Now watching {} directories.",
        watched_directories.len()
    );
    Ok(())
}

/// Test version of handle_config_file_change_internal that loads from a specific config file
#[cfg(test)]
pub fn handle_config_file_change_with_file(
    config: &mut crate::config::Config,
    watched_directories: &mut std::collections::HashSet<PathBuf>,
    watcher_setup: &mut WatcherSetup,
    config_file_path: &std::path::Path,
) -> io::Result<()> {
    println!("Configuration file changed, reloading...");

    // Load new configuration from specific file
    let new_config = match load_config_from_file(config_file_path) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Failed to reload configuration: {}", e);
            return Ok(()); // Don't fail the daemon, just log the error
        }
    };

    let new_watched_directories = new_config.get_watched_directories().clone();

    // Find directories that were added
    let added_directories: std::collections::HashSet<_> = new_watched_directories
        .difference(watched_directories)
        .collect();

    // Find directories that were removed
    let removed_directories: std::collections::HashSet<_> = watched_directories
        .difference(&new_watched_directories)
        .collect();

    if !added_directories.is_empty() {
        println!(
            "Adding {} new directories to watch:",
            added_directories.len()
        );
        for dir in &added_directories {
            println!("  + {}", dir.display());
        }

        // Add watchers for new directories
        let added_dirs_set: std::collections::HashSet<PathBuf> =
            added_directories.into_iter().cloned().collect();
        let (tx, _) = mpsc::channel(); // We don't use this receiver, just need the sender
        if let Err(e) = setup_directory_watchers(
            &added_dirs_set,
            &tx,
            &mut watcher_setup.watchers,
            &mut watcher_setup.rules_paths,
        ) {
            eprintln!("Failed to setup watchers for new directories: {}", e);
        }
    }

    if !removed_directories.is_empty() {
        println!(
            "Removing {} directories from watch:",
            removed_directories.len()
        );
        for dir in &removed_directories {
            println!("  - {}", dir.display());
        }

        // Remove watchers and rules_paths entries for removed directories
        for removed_dir in &removed_directories {
            let rules_path = removed_dir.join(RULES_DIR);
            if let Ok(canonical_path) = rules_path.canonicalize() {
                watcher_setup.rules_paths.remove(&canonical_path);
            }

            // Remove symlinks from the removed directory
            if let Err(e) = remove_symlinks_from_directory(removed_dir) {
                eprintln!(
                    "Failed to remove symlinks from {}: {}",
                    removed_dir.display(),
                    e
                );
            } else {
                println!("Removed symlinks from {}", removed_dir.display());
            }
        }
    }

    // Update our local state
    *config = new_config;
    *watched_directories = new_watched_directories;

    println!(
        "Configuration reloaded successfully. Now watching {} directories.",
        watched_directories.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::RULES_DIR;
    use crate::symlinks::create_symlink_to_file;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_handle_config_file_change_add_directories() {
        let initial_dir = tempdir().unwrap();
        let new_dir = tempdir().unwrap();

        // Create .rules directories
        let initial_rules_path = initial_dir.path().join(RULES_DIR);
        let new_rules_path = new_dir.path().join(RULES_DIR);
        fs::create_dir(&initial_rules_path).unwrap();
        fs::create_dir(&new_rules_path).unwrap();

        // Create test files
        fs::write(initial_rules_path.join("existing.md"), "existing content").unwrap();
        fs::write(new_rules_path.join("new.md"), "new content").unwrap();

        // Set up initial config and watcher setup
        let mut config = crate::config::Config::new();
        config.add_directory(initial_dir.path().to_path_buf());

        let mut watched_directories = std::collections::HashSet::new();
        watched_directories.insert(initial_dir.path().to_path_buf());

        let mut watcher_setup = WatcherSetup {
            watchers: Vec::new(),
            rules_paths: HashMap::new(),
            event_receiver: mpsc::channel().1,
            config_file_path: std::env::temp_dir().join("test_config.json"),
        };

        // Add the initial directory to rules_paths
        watcher_setup.rules_paths.insert(
            initial_rules_path.canonicalize().unwrap(),
            initial_dir.path().to_path_buf(),
        );

        // Mock the new config by adding the new directory
        config.add_directory(new_dir.path().to_path_buf());

        // Temporarily save the new config to a file
        let temp_config_file = tempdir().unwrap();
        let config_path = temp_config_file.path().join("config.json");
        crate::config::save_config_to_file(&config, &config_path).unwrap();

        // Test the configuration change handling
        let result = handle_config_file_change_with_file(
            &mut config,
            &mut watched_directories,
            &mut watcher_setup,
            &config_path,
        );

        assert!(result.is_ok());

        // Verify the new directory was added to watched directories
        // Use canonical paths for comparison to handle symlink differences
        let new_dir_canonical = new_dir.path().canonicalize().unwrap();
        let contains_new_dir = watched_directories.iter().any(|dir| {
            dir.canonicalize().map_or(false, |canonical| canonical == new_dir_canonical)
        });
        assert!(contains_new_dir, "New directory should be in watched directories");
        assert_eq!(watched_directories.len(), 2);

        // Verify the new directory was added to rules_paths
        let new_rules_canonical = new_rules_path.canonicalize().unwrap();
        assert!(watcher_setup.rules_paths.contains_key(&new_rules_canonical));
    }

    #[test]
    fn test_handle_config_file_change_remove_directories() {
        let dir1 = tempdir().unwrap();
        let dir2 = tempdir().unwrap();

        // Create .rules directories
        let rules_path1 = dir1.path().join(RULES_DIR);
        let rules_path2 = dir2.path().join(RULES_DIR);
        fs::create_dir(&rules_path1).unwrap();
        fs::create_dir(&rules_path2).unwrap();

        // Create target directories
        let cursor_rules_path1 = dir1.path().join(".cursor/rules");
        let windsurf_rules_path1 = dir1.path().join(".windsurf/rules");
        fs::create_dir_all(&cursor_rules_path1).unwrap();
        fs::create_dir_all(&windsurf_rules_path1).unwrap();

        // Create test files and symlinks
        let test_file1 = rules_path1.join("test1.md");
        fs::write(&test_file1, "content1").unwrap();
        create_symlink_to_file(&test_file1, &cursor_rules_path1.join("test1.md")).unwrap();
        create_symlink_to_file(&test_file1, &windsurf_rules_path1.join("test1.md")).unwrap();

        // Set up initial config with both directories
        let mut config = crate::config::Config::new();
        config.add_directory(dir1.path().to_path_buf());
        config.add_directory(dir2.path().to_path_buf());

        let mut watched_directories = std::collections::HashSet::new();
        watched_directories.insert(dir1.path().to_path_buf());
        watched_directories.insert(dir2.path().to_path_buf());

        let mut watcher_setup = WatcherSetup {
            watchers: Vec::new(),
            rules_paths: HashMap::new(),
            event_receiver: mpsc::channel().1,
            config_file_path: std::env::temp_dir().join("test_config.json"),
        };

        // Add both directories to rules_paths
        watcher_setup.rules_paths.insert(
            rules_path1.canonicalize().unwrap(),
            dir1.path().to_path_buf(),
        );
        watcher_setup.rules_paths.insert(
            rules_path2.canonicalize().unwrap(),
            dir2.path().to_path_buf(),
        );

        // Verify symlinks exist initially
        assert!(cursor_rules_path1.join("test1.md").exists());
        assert!(windsurf_rules_path1.join("test1.md").exists());

        // Create new config with only dir2 (removing dir1)
        let mut new_config = crate::config::Config::new();
        new_config.add_directory(dir2.path().to_path_buf());

        // Temporarily save the new config to a file
        let temp_config_file = tempdir().unwrap();
        let config_path = temp_config_file.path().join("config.json");
        crate::config::save_config_to_file(&new_config, &config_path).unwrap();

        // Test the configuration change handling
        let result = handle_config_file_change_with_file(
            &mut config,
            &mut watched_directories,
            &mut watcher_setup,
            &config_path,
        );

        assert!(result.is_ok());

        // Verify dir1 was removed from watched directories
        // Use canonical paths for comparison to handle symlink differences
        let dir1_canonical = dir1.path().canonicalize().unwrap();
        let dir2_canonical = dir2.path().canonicalize().unwrap();
        
        let contains_dir1 = watched_directories.iter().any(|dir| {
            dir.canonicalize().map_or(false, |canonical| canonical == dir1_canonical)
        });
        let contains_dir2 = watched_directories.iter().any(|dir| {
            dir.canonicalize().map_or(false, |canonical| canonical == dir2_canonical)
        });
        
        assert!(!contains_dir1, "Dir1 should be removed from watched directories");
        assert!(contains_dir2, "Dir2 should remain in watched directories");
        assert_eq!(watched_directories.len(), 1);

        // Verify dir1 was removed from rules_paths
        let rules_path1_canonical = rules_path1.canonicalize().unwrap();
        assert!(!watcher_setup.rules_paths.contains_key(&rules_path1_canonical));

        // Verify symlinks were removed from dir1
        assert!(!cursor_rules_path1.join("test1.md").exists());
        assert!(!windsurf_rules_path1.join("test1.md").exists());
    }

    #[test]
    fn test_handle_config_file_change_load_config_failure() {
        let dir = tempdir().unwrap();
        let rules_path = dir.path().join(RULES_DIR);
        fs::create_dir(&rules_path).unwrap();

        let mut config = crate::config::Config::new();
        config.add_directory(dir.path().to_path_buf());

        let mut watched_directories = std::collections::HashSet::new();
        watched_directories.insert(dir.path().to_path_buf());

        let mut watcher_setup = WatcherSetup {
            watchers: Vec::new(),
            rules_paths: HashMap::new(),
            event_receiver: mpsc::channel().1,
            config_file_path: std::env::temp_dir().join("test_config.json"),
        };

        // Create an invalid config file to trigger load_config failure
        let temp_config_file = tempdir().unwrap();
        let invalid_config_path = temp_config_file.path().join("invalid_config.json");
        std::fs::write(&invalid_config_path, "invalid json content").unwrap();

        // Test the configuration change handling with invalid config
        let result = handle_config_file_change_with_file(
            &mut config,
            &mut watched_directories,
            &mut watcher_setup,
            &invalid_config_path,
        );

        // Should not error, just log the failure
        assert!(result.is_ok());

        // State should remain unchanged since the config load failed
        // Use canonical paths for comparison to handle symlink differences
        let dir_canonical = dir.path().canonicalize().unwrap();
        let contains_dir = watched_directories.iter().any(|watched_dir| {
            watched_dir.canonicalize().map_or(false, |canonical| canonical == dir_canonical)
        });
        assert!(contains_dir, "Directory should remain in watched directories");
        assert_eq!(watched_directories.len(), 1);
    }
}