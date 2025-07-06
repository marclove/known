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

use crate::config::{get_config_file_path, load_config};
use crate::constants::{CURSOR_RULES_DIR, RULES_DIR, WINDSURF_RULES_DIR};
use crate::single_instance::SingleInstanceLock;
use crate::symlinks::create_symlink_to_file;

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
    // Load configuration to get directories to watch
    let config = load_config()?;
    start_daemon_with_config(shutdown_rx, config)
}

/// Starts a daemon with a custom configuration (for testing)
/// This version skips the single instance lock to allow parallel testing
#[cfg(test)]
pub fn start_daemon_with_test_config(
    shutdown_rx: mpsc::Receiver<()>,
    config: crate::config::Config,
) -> io::Result<()> {
    start_daemon_with_config_no_lock(shutdown_rx, config)
}

/// Represents the complete watcher setup for the daemon.
struct WatcherSetup {
    watchers: Vec<RecommendedWatcher>,
    rules_paths: HashMap<PathBuf, PathBuf>,
    event_receiver: mpsc::Receiver<Result<Event, notify::Error>>,
    config_file_path: PathBuf,
}

/// Prints the list of directories being watched.
fn print_watched_directories(watched_directories: &std::collections::HashSet<PathBuf>) {
    println!(
        "Watching {} directories for changes:",
        watched_directories.len()
    );
    for dir in watched_directories {
        println!("  - {}", dir.display());
    }
}

/// Sets up all watchers (config file watcher and directory watchers).
fn setup_all_watchers(
    watched_directories: &std::collections::HashSet<PathBuf>,
) -> io::Result<WatcherSetup> {
    // Create a map to track rules paths and their canonical versions
    let mut rules_paths = HashMap::new();
    let mut watchers = Vec::new();

    // Set up file watchers for each directory
    let (tx, rx) = mpsc::channel();

    // Watch the configuration file for changes
    let config_file_path = get_config_file_path()?;
    let mut config_watcher = RecommendedWatcher::new(tx.clone(), Config::default())
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    if let Some(config_parent) = config_file_path.parent() {
        if config_parent.exists() {
            config_watcher
                .watch(config_parent, RecursiveMode::NonRecursive)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            println!(
                "Watching configuration file for changes: {}",
                config_file_path.display()
            );
        }
    }

    // Set up watchers for initial directories
    setup_directory_watchers(watched_directories, &tx, &mut watchers, &mut rules_paths)?;

    if watchers.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "No valid .rules directories found to watch",
        ));
    }

    Ok(WatcherSetup {
        watchers,
        rules_paths,
        event_receiver: rx,
        config_file_path,
    })
}

/// Runs the main daemon event loop.
fn run_daemon_event_loop(
    shutdown_rx: mpsc::Receiver<()>,
    config: &mut crate::config::Config,
    watched_directories: &mut std::collections::HashSet<PathBuf>,
    mut watcher_setup: WatcherSetup,
) -> io::Result<()> {
    // Main event loop
    loop {
        // Check for shutdown signal (non-blocking)
        if let Ok(()) = shutdown_rx.try_recv() {
            println!("Daemon shutdown requested");
            break;
        }

        // Check for file system events (with timeout)
        match watcher_setup
            .event_receiver
            .recv_timeout(Duration::from_millis(100))
        {
            Ok(Ok(event)) => {
                // Check if this is a config file change
                if is_config_file_event(&event, &watcher_setup.config_file_path) {
                    if let Err(e) = handle_config_file_change_internal(
                        config,
                        watched_directories,
                        &mut watcher_setup,
                    ) {
                        eprintln!("Error handling config file change: {}", e);
                    }
                } else if let Err(e) = handle_file_event(&event, &watcher_setup.rules_paths) {
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

    Ok(())
}

/// Handles configuration file changes with watcher setup management.
fn handle_config_file_change_internal(
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

/// Internal function that handles the actual daemon logic with a given config
fn start_daemon_with_config(
    shutdown_rx: mpsc::Receiver<()>,
    config: crate::config::Config,
) -> io::Result<()> {
    // Acquire system-wide single instance lock first
    let _lock = SingleInstanceLock::acquire()?;
    println!("Acquired system-wide single instance lock");

    start_daemon_with_config_no_lock(shutdown_rx, config)
}

/// Internal function that handles the daemon logic without acquiring a lock (for testing)
fn start_daemon_with_config_no_lock(
    shutdown_rx: mpsc::Receiver<()>,
    mut config: crate::config::Config,
) -> io::Result<()> {
    let mut watched_directories = config.get_watched_directories().clone();

    if watched_directories.is_empty() {
        println!("No directories configured to watch. Use 'known symlink' in project directories to add them.");
        return Ok(());
    }

    print_watched_directories(&watched_directories);

    let watcher_setup = setup_all_watchers(&watched_directories)?;

    println!(
        "System-wide daemon started, watching {} directories for changes...",
        watcher_setup.watchers.len()
    );

    run_daemon_event_loop(
        shutdown_rx,
        &mut config,
        &mut watched_directories,
        watcher_setup,
    )?;

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

/// Removes all symlinks from the target directories for a given project directory.
///
/// This function removes all symlinks in both .cursor/rules and .windsurf/rules
/// directories for the specified project directory. It only removes the symlinks,
/// not the original files in .rules.
///
/// # Arguments
///
/// * `dir` - Path to the project directory containing the .rules directory
///
/// # Errors
///
/// Returns an error if directory operations or file removal fails
///
fn remove_symlinks_from_directory(dir: &Path) -> io::Result<()> {
    let cursor_rules_path = dir.join(CURSOR_RULES_DIR);
    let windsurf_rules_path = dir.join(WINDSURF_RULES_DIR);

    // Remove all files from .cursor/rules if it exists
    if cursor_rules_path.exists() {
        for entry in fs::read_dir(&cursor_rules_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                fs::remove_file(&path)?;
            }
        }
    }

    // Remove all files from .windsurf/rules if it exists
    if windsurf_rules_path.exists() {
        for entry in fs::read_dir(&windsurf_rules_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                fs::remove_file(&path)?;
            }
        }
    }

    Ok(())
}

/// Sets up watchers for the given directories
fn setup_directory_watchers(
    directories: &std::collections::HashSet<PathBuf>,
    tx: &mpsc::Sender<Result<Event, notify::Error>>,
    watchers: &mut Vec<RecommendedWatcher>,
    rules_paths: &mut HashMap<PathBuf, PathBuf>,
) -> io::Result<()> {
    for dir in directories {
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
    Ok(())
}

/// Checks if the file event is related to the configuration file
fn is_config_file_event(event: &Event, config_file_path: &Path) -> bool {
    event.paths.iter().any(|path| {
        path.file_name() == config_file_path.file_name()
            && path.parent() == config_file_path.parent()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
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
    fn test_daemon_no_configured_directories() {
        // Create empty config (no need to save to file system)
        let config = Config::new();

        // Create channel for shutdown signal
        let (_shutdown_tx, shutdown_rx) = mpsc::channel();

        // Start daemon with test config - should complete immediately with no directories
        let result = start_daemon_with_test_config(shutdown_rx, config);
        assert!(
            result.is_ok(),
            "Daemon should handle empty config gracefully"
        );
    }

    #[test]
    fn test_is_config_file_event() {
        let config_path = Path::new("/home/user/.config/known/config.json");

        // Test event that matches config file
        let matching_event = Event {
            kind: EventKind::Modify(ModifyKind::Data(notify::event::DataChange::Content)),
            paths: vec![config_path.to_path_buf()],
            attrs: Default::default(),
        };

        assert!(is_config_file_event(&matching_event, config_path));

        // Test event that doesn't match config file
        let non_matching_event = Event {
            kind: EventKind::Modify(ModifyKind::Data(notify::event::DataChange::Content)),
            paths: vec![Path::new("/some/other/file.txt").to_path_buf()],
            attrs: Default::default(),
        };

        assert!(!is_config_file_event(&non_matching_event, config_path));
    }

    #[test]
    fn test_setup_directory_watchers() {
        let dir1 = tempdir().unwrap();
        let dir2 = tempdir().unwrap();

        // Create .rules directories
        let rules_path1 = dir1.path().join(RULES_DIR);
        let rules_path2 = dir2.path().join(RULES_DIR);
        fs::create_dir(&rules_path1).unwrap();
        fs::create_dir(&rules_path2).unwrap();

        // Create test files
        fs::write(rules_path1.join("test1.md"), "content1").unwrap();
        fs::write(rules_path2.join("test2.md"), "content2").unwrap();

        // Create directory set
        let mut directories = std::collections::HashSet::new();
        directories.insert(dir1.path().to_path_buf());
        directories.insert(dir2.path().to_path_buf());

        // Set up watchers
        let (tx, _rx) = mpsc::channel();
        let mut watchers = Vec::new();
        let mut rules_paths = HashMap::new();

        let result = setup_directory_watchers(&directories, &tx, &mut watchers, &mut rules_paths);
        assert!(result.is_ok(), "Should successfully set up watchers");

        // Verify watchers were created
        assert_eq!(watchers.len(), 2, "Should have 2 watchers");

        // Verify rules_paths contains both directories
        assert_eq!(rules_paths.len(), 2, "Should track 2 rules paths");

        // Verify symlinks were created
        assert!(dir1.path().join(CURSOR_RULES_DIR).join("test1.md").exists());
        assert!(dir1
            .path()
            .join(WINDSURF_RULES_DIR)
            .join("test1.md")
            .exists());
        assert!(dir2.path().join(CURSOR_RULES_DIR).join("test2.md").exists());
        assert!(dir2
            .path()
            .join(WINDSURF_RULES_DIR)
            .join("test2.md")
            .exists());
    }

    #[test]
    fn test_remove_symlinks_when_directory_removed_from_config() {
        let dir1 = tempdir().unwrap();
        let dir2 = tempdir().unwrap();

        // Create .rules directories and files
        let rules_path1 = dir1.path().join(RULES_DIR);
        let rules_path2 = dir2.path().join(RULES_DIR);
        fs::create_dir(&rules_path1).unwrap();
        fs::create_dir(&rules_path2).unwrap();

        fs::write(rules_path1.join("test1.md"), "content1").unwrap();
        fs::write(rules_path2.join("test2.md"), "content2").unwrap();

        // Create target directories
        let cursor_rules_path1 = dir1.path().join(CURSOR_RULES_DIR);
        let windsurf_rules_path1 = dir1.path().join(WINDSURF_RULES_DIR);
        let cursor_rules_path2 = dir2.path().join(CURSOR_RULES_DIR);
        let windsurf_rules_path2 = dir2.path().join(WINDSURF_RULES_DIR);

        // Sync both directories to create initial symlinks
        sync_rules_directory(&rules_path1, &cursor_rules_path1, &windsurf_rules_path1).unwrap();
        sync_rules_directory(&rules_path2, &cursor_rules_path2, &windsurf_rules_path2).unwrap();

        // Verify symlinks exist initially
        assert!(cursor_rules_path1.join("test1.md").exists());
        assert!(windsurf_rules_path1.join("test1.md").exists());
        assert!(cursor_rules_path2.join("test2.md").exists());
        assert!(windsurf_rules_path2.join("test2.md").exists());

        // Remove symlinks from dir2 (simulating removal from config)
        remove_symlinks_from_directory(dir2.path()).unwrap();

        // Verify symlinks in dir1 still exist
        assert!(cursor_rules_path1.join("test1.md").exists());
        assert!(windsurf_rules_path1.join("test1.md").exists());

        // Verify symlinks in dir2 have been removed
        assert!(!cursor_rules_path2.join("test2.md").exists());
        assert!(!windsurf_rules_path2.join("test2.md").exists());

        // Verify original files in .rules are untouched
        assert!(rules_path1.join("test1.md").exists());
        assert!(rules_path2.join("test2.md").exists());
    }

    #[test]
    fn test_start_daemon_with_config_no_lock_comprehensive() {
        // This test verifies comprehensive behavior of start_daemon_with_config_no_lock
        use std::time::Duration;

        let dir1 = tempdir().unwrap();
        let dir2 = tempdir().unwrap();

        // Create .rules directories with test files
        let rules_path1 = dir1.path().join(RULES_DIR);
        let rules_path2 = dir2.path().join(RULES_DIR);
        fs::create_dir(&rules_path1).unwrap();
        fs::create_dir(&rules_path2).unwrap();
        fs::write(rules_path1.join("test1.md"), "content1").unwrap();
        fs::write(rules_path2.join("test2.md"), "content2").unwrap();

        // Create config with both directories
        let mut config = Config::new();
        config.add_directory(dir1.path());
        config.add_directory(dir2.path());

        // Test 1: Daemon should set up watchers and create initial symlinks
        let (shutdown_tx, shutdown_rx) = mpsc::channel();

        // Start daemon in a separate thread that will exit quickly
        let config_clone = config.clone();
        let handle =
            std::thread::spawn(move || start_daemon_with_test_config(shutdown_rx, config_clone));

        // Give daemon time to initialize
        std::thread::sleep(Duration::from_millis(50));

        // Send shutdown signal
        shutdown_tx.send(()).unwrap();

        // Wait for daemon to finish
        let result = handle.join().unwrap();
        assert!(
            result.is_ok(),
            "Daemon should start and shutdown successfully"
        );

        // Verify that symlinks were created for both directories
        assert!(dir1.path().join(CURSOR_RULES_DIR).join("test1.md").exists());
        assert!(dir1
            .path()
            .join(WINDSURF_RULES_DIR)
            .join("test1.md")
            .exists());
        assert!(dir2.path().join(CURSOR_RULES_DIR).join("test2.md").exists());
        assert!(dir2
            .path()
            .join(WINDSURF_RULES_DIR)
            .join("test2.md")
            .exists());

        // Test 2: Empty config should return Ok without error
        let empty_config = Config::new();
        let (_shutdown_tx2, shutdown_rx2) = mpsc::channel();
        let result2 = start_daemon_with_test_config(shutdown_rx2, empty_config);
        assert!(
            result2.is_ok(),
            "Daemon should handle empty config gracefully"
        );
    }
}
