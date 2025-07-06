//! File watching daemon functionality for managing symlinks in rules directories.

use notify::{
    event::{ModifyKind, RenameMode},
    Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use crate::config::load_config;
use crate::single_instance::SingleInstanceLock;
use crate::symlinks::create_symlink_to_file;

/// The directory name for rules files
const RULES_DIR: &str = ".rules";

/// The directory name for cursor rules files
const CURSOR_RULES_DIR: &str = ".cursor/rules";

/// The directory name for windsurf rules files
const WINDSURF_RULES_DIR: &str = ".windsurf/rules";

/// Starts a daemon that watches all configured directories for changes
/// and maintains synchronized symlinks in .cursor/rules and .windsurf/rules directories.
///
/// This function creates a file watcher that monitors all directories configured in the
/// configuration file. It watches each directory's .rules subdirectory for file additions,
/// modifications, and deletions. When changes are detected, it automatically updates
/// the corresponding symlinks in the .cursor/rules and .windsurf/rules directories.
///
/// # Single Instance Enforcement
///
/// Only one instance of the daemon can run at a time system-wide. The function uses a PID
/// file locking mechanism with a centralized lock file to ensure that multiple daemon processes cannot run
/// simultaneously anywhere on the system.
///
/// # Behavior
///
/// - Acquires a system-wide single instance lock using a centralized PID file
/// - Loads the configuration to get all directories to watch
/// - Watches each directory's .rules subdirectory for file system events
/// - Creates symlinks in .cursor/rules and .windsurf/rules for each file in .rules
/// - Removes symlinks when files are deleted from .rules
/// - Runs indefinitely until the receiver channel is closed
/// - Prints status messages to stdout for user feedback
/// - Automatically releases the lock when the daemon stops
///
/// # Arguments
///
/// * `shutdown_rx` - A receiver channel that signals when to stop the daemon
///
/// # Errors
///
/// Returns an error if:
/// - Another instance of the daemon is already running system-wide
/// - Configuration file cannot be loaded
/// - No directories are configured to watch
/// - Watcher creation fails
/// - File system operations fail
/// - Directory creation fails
/// - Unable to determine application directories for this platform
///
pub fn start_daemon(shutdown_rx: mpsc::Receiver<()>) -> io::Result<()> {
    // Acquire system-wide single instance lock first
    let _lock = SingleInstanceLock::acquire()?;
    println!("Acquired system-wide single instance lock");

    // Load configuration to get directories to watch
    let config = load_config()?;
    let watched_directories = config.get_watched_directories();

    if watched_directories.is_empty() {
        println!("No directories configured to watch. Use 'known symlink' in project directories to add them.");
        return Ok(());
    }

    println!(
        "Watching {} directories for changes:",
        watched_directories.len()
    );
    for dir in watched_directories {
        println!("  - {}", dir.display());
    }

    // Create a map to track rules paths and their canonical versions
    let mut rules_paths = HashMap::new();
    let mut watchers = Vec::new();

    // Set up file watchers for each directory
    let (tx, rx) = mpsc::channel();

    for dir in watched_directories {
        let rules_path = dir.join(RULES_DIR);

        // Skip if .rules directory doesn't exist
        if !rules_path.exists() {
            println!(
                "Warning: .rules directory not found at {}, skipping",
                rules_path.display()
            );
            continue;
        }

        // Canonicalize rules path to handle symlinks properly
        let rules_path_canonical = rules_path.canonicalize()?;
        rules_paths.insert(rules_path_canonical.clone(), dir.clone());

        // Create target directories if they don't exist
        let cursor_rules_path = dir.join(CURSOR_RULES_DIR);
        let windsurf_rules_path = dir.join(WINDSURF_RULES_DIR);

        if let Some(parent) = cursor_rules_path.parent() {
            fs::create_dir_all(parent)?;
        }
        if let Some(parent) = windsurf_rules_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Create initial symlinks for existing files
        sync_rules_directory(&rules_path, &cursor_rules_path, &windsurf_rules_path)?;

        // Create watcher for this directory
        let mut watcher = RecommendedWatcher::new(tx.clone(), Config::default())
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        // Watch the .rules directory
        watcher
            .watch(&rules_path, RecursiveMode::NonRecursive)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        watchers.push(watcher);
    }

    if watchers.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "No valid .rules directories found to watch",
        ));
    }

    println!(
        "System-wide daemon started, watching {} directories for changes...",
        watchers.len()
    );

    // Main event loop
    loop {
        // Check for shutdown signal (non-blocking)
        if let Ok(()) = shutdown_rx.try_recv() {
            println!("Daemon shutdown requested");
            break;
        }

        // Check for file system events (with timeout)
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(Ok(event)) => {
                if let Err(e) = handle_file_event(&event, &rules_paths) {
                    eprintln!("Error handling file event: {}", e);
                }
            }
            Ok(Err(e)) => {
                eprintln!("Watch error: {}", e);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // Timeout is expected, continue loop
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                println!("Watcher disconnected, stopping daemon");
                break;
            }
        }
    }

    println!("System-wide daemon stopped");
    Ok(())
}

/// Handles a file system event by updating symlinks in target directories.
///
/// # Arguments
///
/// * `event` - The file system event to handle
/// * `rules_paths` - Map of canonical rules paths to their parent directories
///
/// # Errors
///
/// Returns an error if symlink operations fail
///
fn handle_file_event(event: &Event, rules_paths: &HashMap<PathBuf, PathBuf>) -> io::Result<()> {
    for path in &event.paths {
        // Find which rules directory this event belongs to
        let (_rules_path, parent_dir) = match rules_paths
            .iter()
            .find(|(rules_path, _)| path.starts_with(rules_path))
        {
            Some((rules_path, parent_dir)) => (rules_path, parent_dir),
            None => continue, // Event not in any watched directory
        };

        let file_name = match path.file_name() {
            Some(name) => name,
            None => continue,
        };

        let cursor_rules_path = parent_dir.join(CURSOR_RULES_DIR);
        let windsurf_rules_path = parent_dir.join(WINDSURF_RULES_DIR);
        let cursor_target = cursor_rules_path.join(file_name);
        let windsurf_target = windsurf_rules_path.join(file_name);

        match event.kind {
            EventKind::Create(_) => {
                // Create symlinks for new files
                if path.is_file() {
                    create_symlink_to_file(path, &cursor_target)?;
                    create_symlink_to_file(path, &windsurf_target)?;
                    println!(
                        "Created symlinks for {} in {}",
                        file_name.to_string_lossy(),
                        parent_dir.display()
                    );
                }
            }
            EventKind::Modify(ModifyKind::Name(RenameMode::From)) => {
                // File is being renamed FROM this name - remove old symlinks
                if cursor_target.exists() {
                    fs::remove_file(&cursor_target)?;
                }
                if windsurf_target.exists() {
                    fs::remove_file(&windsurf_target)?;
                }
                println!(
                    "Removed symlinks for renamed file {} in {}",
                    file_name.to_string_lossy(),
                    parent_dir.display()
                );
            }
            EventKind::Modify(ModifyKind::Name(RenameMode::To)) => {
                // File is being renamed TO this name - create new symlinks
                if path.is_file() {
                    create_symlink_to_file(path, &cursor_target)?;
                    create_symlink_to_file(path, &windsurf_target)?;
                    println!(
                        "Created symlinks for renamed file {} in {}",
                        file_name.to_string_lossy(),
                        parent_dir.display()
                    );
                }
            }
            EventKind::Modify(_) => {
                // Other modifications (content changes, metadata) - update symlinks if file exists
                if path.is_file() {
                    create_symlink_to_file(path, &cursor_target)?;
                    create_symlink_to_file(path, &windsurf_target)?;
                    println!(
                        "Updated symlinks for {} in {}",
                        file_name.to_string_lossy(),
                        parent_dir.display()
                    );
                }
            }
            EventKind::Remove(_) => {
                // Remove symlinks
                if cursor_target.exists() {
                    fs::remove_file(&cursor_target)?;
                }
                if windsurf_target.exists() {
                    fs::remove_file(&windsurf_target)?;
                }
                println!(
                    "Removed symlinks for {} in {}",
                    file_name.to_string_lossy(),
                    parent_dir.display()
                );
            }
            _ => {
                // Ignore other event types
            }
        }
    }
    Ok(())
}

/// Synchronizes the rules directory with target directories by creating symlinks.
///
/// This function scans the .rules directory and creates symlinks in both
/// .cursor/rules and .windsurf/rules directories for each file found.
///
/// # Arguments
///
/// * `rules_path` - Path to the .rules directory
/// * `cursor_rules_path` - Path to the .cursor/rules directory
/// * `windsurf_rules_path` - Path to the .windsurf/rules directory
///
/// # Errors
///
/// Returns an error if directory operations or symlink creation fails
///
fn sync_rules_directory(
    rules_path: &Path,
    cursor_rules_path: &Path,
    windsurf_rules_path: &Path,
) -> io::Result<()> {
    // Create target directories if they don't exist
    fs::create_dir_all(cursor_rules_path)?;
    fs::create_dir_all(windsurf_rules_path)?;

    // Create symlinks for all existing files in .rules
    for entry in fs::read_dir(rules_path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            let file_name = path.file_name().unwrap();
            let cursor_target = cursor_rules_path.join(file_name);
            let windsurf_target = windsurf_rules_path.join(file_name);

            create_symlink_to_file(&path, &cursor_target)?;
            create_symlink_to_file(&path, &windsurf_target)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{save_config, Config};
    use tempfile::tempdir;

    #[test]
    fn test_sync_rules_directory() {
        let dir = tempdir().unwrap();

        // Create .rules directory with test files
        let rules_path = dir.path().join(RULES_DIR);
        fs::create_dir(&rules_path).unwrap();

        let test_file1 = rules_path.join("test1.md");
        let test_file2 = rules_path.join("test2.txt");
        fs::write(&test_file1, "Test content 1").unwrap();
        fs::write(&test_file2, "Test content 2").unwrap();

        // Create target directories
        let cursor_rules_path = dir.path().join(CURSOR_RULES_DIR);
        let windsurf_rules_path = dir.path().join(WINDSURF_RULES_DIR);

        // Call sync_rules_directory
        sync_rules_directory(&rules_path, &cursor_rules_path, &windsurf_rules_path).unwrap();

        // Verify symlinks were created
        let cursor_symlink1 = cursor_rules_path.join("test1.md");
        let cursor_symlink2 = cursor_rules_path.join("test2.txt");
        let windsurf_symlink1 = windsurf_rules_path.join("test1.md");
        let windsurf_symlink2 = windsurf_rules_path.join("test2.txt");

        assert!(cursor_symlink1.exists(), "Cursor symlink 1 should exist");
        assert!(cursor_symlink2.exists(), "Cursor symlink 2 should exist");
        assert!(
            windsurf_symlink1.exists(),
            "Windsurf symlink 1 should exist"
        );
        assert!(
            windsurf_symlink2.exists(),
            "Windsurf symlink 2 should exist"
        );

        // Verify symlinks point to correct content
        assert_eq!(
            fs::read_to_string(&cursor_symlink1).unwrap(),
            "Test content 1"
        );
        assert_eq!(
            fs::read_to_string(&cursor_symlink2).unwrap(),
            "Test content 2"
        );
        assert_eq!(
            fs::read_to_string(&windsurf_symlink1).unwrap(),
            "Test content 1"
        );
        assert_eq!(
            fs::read_to_string(&windsurf_symlink2).unwrap(),
            "Test content 2"
        );
    }

    #[test]
    fn test_handle_file_event_with_multiple_directories() {
        use std::collections::HashMap;

        let dir1 = tempdir().unwrap();
        let dir2 = tempdir().unwrap();

        // Create .rules directories
        let rules_path1 = dir1.path().join(RULES_DIR);
        let rules_path2 = dir2.path().join(RULES_DIR);
        fs::create_dir(&rules_path1).unwrap();
        fs::create_dir(&rules_path2).unwrap();

        // Create target directories
        let cursor_rules_path1 = dir1.path().join(CURSOR_RULES_DIR);
        let windsurf_rules_path1 = dir1.path().join(WINDSURF_RULES_DIR);
        let cursor_rules_path2 = dir2.path().join(CURSOR_RULES_DIR);
        let windsurf_rules_path2 = dir2.path().join(WINDSURF_RULES_DIR);
        fs::create_dir_all(&cursor_rules_path1).unwrap();
        fs::create_dir_all(&windsurf_rules_path1).unwrap();
        fs::create_dir_all(&cursor_rules_path2).unwrap();
        fs::create_dir_all(&windsurf_rules_path2).unwrap();

        // Create test files
        let test_file1 = rules_path1.join("test1.md");
        let test_file2 = rules_path2.join("test2.md");
        fs::write(&test_file1, "Test content 1").unwrap();
        fs::write(&test_file2, "Test content 2").unwrap();

        // Create rules paths map
        let mut rules_paths = HashMap::new();
        rules_paths.insert(
            rules_path1.canonicalize().unwrap(),
            dir1.path().to_path_buf(),
        );
        rules_paths.insert(
            rules_path2.canonicalize().unwrap(),
            dir2.path().to_path_buf(),
        );

        // Simulate create events for both files
        let event1 = Event {
            kind: EventKind::Create(notify::event::CreateKind::File),
            paths: vec![test_file1.canonicalize().unwrap()],
            attrs: Default::default(),
        };

        let event2 = Event {
            kind: EventKind::Create(notify::event::CreateKind::File),
            paths: vec![test_file2.canonicalize().unwrap()],
            attrs: Default::default(),
        };

        // Handle events
        handle_file_event(&event1, &rules_paths).unwrap();
        handle_file_event(&event2, &rules_paths).unwrap();

        // Verify symlinks were created in correct directories
        assert!(cursor_rules_path1.join("test1.md").exists());
        assert!(windsurf_rules_path1.join("test1.md").exists());
        assert!(cursor_rules_path2.join("test2.md").exists());
        assert!(windsurf_rules_path2.join("test2.md").exists());

        // Verify symlinks don't exist in wrong directories
        assert!(!cursor_rules_path1.join("test2.md").exists());
        assert!(!cursor_rules_path2.join("test1.md").exists());
    }

    #[test]
    #[serial_test::serial]
    fn test_daemon_no_configured_directories() {
        // Create empty config
        let config = Config::new();

        // Temporarily save empty config for test
        let original_config = crate::config::load_config().unwrap_or_default();
        save_config(&config).unwrap();

        // Create channel for shutdown signal
        let (_shutdown_tx, shutdown_rx) = mpsc::channel();

        // Start daemon - should complete immediately with no directories
        let result = start_daemon(shutdown_rx);
        assert!(
            result.is_ok(),
            "Daemon should handle empty config gracefully"
        );

        // Restore original config
        save_config(&original_config).unwrap();
    }
}
