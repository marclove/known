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

/// Starts a system-wide daemon that watches all configured directories for changes
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
pub fn start_system_daemon(shutdown_rx: mpsc::Receiver<()>) -> io::Result<()> {
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
                if let Err(e) = handle_system_file_event(&event, &rules_paths) {
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

/// Starts a daemon that watches the .rules directory for changes and maintains
/// synchronized symlinks in .cursor/rules and .windsurf/rules directories.
///
/// This function creates a file watcher that monitors the .rules directory for
/// file additions, modifications, and deletions. When changes are detected,
/// it automatically updates the corresponding symlinks in the .cursor/rules
/// and .windsurf/rules directories to keep them in sync.
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
/// - Watches the .rules directory recursively for file system events
/// - Creates symlinks in .cursor/rules and .windsurf/rules for each file in .rules
/// - Removes symlinks when files are deleted from .rules
/// - Runs indefinitely until the receiver channel is closed
/// - Prints status messages to stdout for user feedback
/// - Automatically releases the lock when the daemon stops
///
/// # Arguments
///
/// * `dir` - The directory path containing the .rules directory to watch
/// * `shutdown_rx` - A receiver channel that signals when to stop the daemon
///
/// # Errors
///
/// Returns an error if:
/// - Another instance of the daemon is already running system-wide
/// - The .rules directory doesn't exist
/// - Watcher creation fails
/// - File system operations fail
/// - Directory creation fails
/// - Unable to determine application directories for this platform
///
/// # Deprecated
///
/// This function is deprecated in favor of `start_system_daemon` which watches
/// all configured directories from the configuration file.
pub fn start_daemon<P: AsRef<Path>>(dir: P, shutdown_rx: mpsc::Receiver<()>) -> io::Result<()> {
    let dir = dir.as_ref();
    let rules_path = dir.join(RULES_DIR);

    // Acquire system-wide single instance lock first
    let _lock = SingleInstanceLock::acquire()?;
    println!("Acquired system-wide single instance lock");

    // Check if .rules directory exists
    if !rules_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(".rules directory not found at {}", rules_path.display()),
        ));
    }

    // Canonicalize rules path to handle symlinks properly
    let rules_path_canonical = rules_path.canonicalize()?;

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

    // Set up file watcher
    let (tx, rx) = mpsc::channel();
    let mut watcher = RecommendedWatcher::new(tx, Config::default())
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    // Watch the .rules directory
    watcher
        .watch(&rules_path, RecursiveMode::NonRecursive)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    println!(
        "Daemon started, watching {} for changes...",
        rules_path.display()
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
                if let Err(e) = handle_file_event(
                    &event,
                    &rules_path_canonical,
                    &cursor_rules_path,
                    &windsurf_rules_path,
                ) {
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

    println!("Daemon stopped");
    Ok(())
}

/// Handles a file system event for the system-wide daemon by updating symlinks in target directories.
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
fn handle_system_file_event(
    event: &Event,
    rules_paths: &HashMap<PathBuf, PathBuf>,
) -> io::Result<()> {
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

/// Handles a file system event by updating symlinks in target directories.
///
/// # Arguments
///
/// * `event` - The file system event to handle
/// * `rules_path` - Path to the .rules directory
/// * `cursor_rules_path` - Path to the .cursor/rules directory
/// * `windsurf_rules_path` - Path to the .windsurf/rules directory
///
/// # Errors
///
/// Returns an error if symlink operations fail
///
fn handle_file_event(
    event: &Event,
    rules_path: &Path,
    cursor_rules_path: &Path,
    windsurf_rules_path: &Path,
) -> io::Result<()> {
    for path in &event.paths {
        // Only handle files within the .rules directory
        if !path.starts_with(rules_path) {
            continue;
        }

        let file_name = match path.file_name() {
            Some(name) => name,
            None => continue,
        };

        let cursor_target = cursor_rules_path.join(file_name);
        let windsurf_target = windsurf_rules_path.join(file_name);

        match event.kind {
            EventKind::Create(_) => {
                // Create symlinks for new files
                if path.is_file() {
                    create_symlink_to_file(path, &cursor_target)?;
                    create_symlink_to_file(path, &windsurf_target)?;
                    println!("Created symlinks for {}", file_name.to_string_lossy());
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
                    "Removed symlinks for renamed file {}",
                    file_name.to_string_lossy()
                );
            }
            EventKind::Modify(ModifyKind::Name(RenameMode::To)) => {
                // File is being renamed TO this name - create new symlinks
                if path.is_file() {
                    create_symlink_to_file(path, &cursor_target)?;
                    create_symlink_to_file(path, &windsurf_target)?;
                    println!(
                        "Created symlinks for renamed file {}",
                        file_name.to_string_lossy()
                    );
                }
            }
            EventKind::Modify(_) => {
                // Other modifications (content changes, metadata) - update symlinks if file exists
                if path.is_file() {
                    create_symlink_to_file(path, &cursor_target)?;
                    create_symlink_to_file(path, &windsurf_target)?;
                    println!("Updated symlinks for {}", file_name.to_string_lossy());
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
                println!("Removed symlinks for {}", file_name.to_string_lossy());
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
    use std::thread;
    use tempfile::tempdir;

    #[test]
    #[serial_test::serial]
    fn test_daemon_watches_rules_directory() {
        let dir = tempdir().unwrap();

        // Create .rules directory
        let rules_path = dir.path().join(RULES_DIR);
        fs::create_dir(&rules_path).unwrap();

        // Create target directories
        let cursor_rules_path = dir.path().join(CURSOR_RULES_DIR);
        let windsurf_rules_path = dir.path().join(WINDSURF_RULES_DIR);
        fs::create_dir_all(&cursor_rules_path).unwrap();
        fs::create_dir_all(&windsurf_rules_path).unwrap();

        // Create channel for shutdown signal
        let (shutdown_tx, shutdown_rx) = mpsc::channel();

        // Add a file to .rules directory BEFORE starting daemon
        let test_file = rules_path.join("test.md");
        fs::write(&test_file, "# Test content").unwrap();

        // Start daemon in background thread
        let daemon_dir = dir.path().to_path_buf();
        let daemon_handle = thread::spawn(move || start_daemon(daemon_dir, shutdown_rx));

        // Give daemon time to start and sync existing files
        thread::sleep(Duration::from_millis(300));

        // Verify symlinks were created in target directories for existing file
        let cursor_symlink = cursor_rules_path.join("test.md");
        let windsurf_symlink = windsurf_rules_path.join("test.md");
        assert!(cursor_symlink.exists(), "Cursor symlink should exist");
        assert!(windsurf_symlink.exists(), "Windsurf symlink should exist");

        // Verify symlinks point to correct content
        let cursor_content = fs::read_to_string(&cursor_symlink).unwrap();
        let windsurf_content = fs::read_to_string(&windsurf_symlink).unwrap();
        assert_eq!(cursor_content, "# Test content");
        assert_eq!(windsurf_content, "# Test content");

        // Test adding a new file after daemon start
        let test_file2 = rules_path.join("test2.md");
        fs::write(&test_file2, "# Test content 2").unwrap();

        // Give daemon time to process the new file
        thread::sleep(Duration::from_millis(300));

        // Verify symlinks were created for new file
        let cursor_symlink2 = cursor_rules_path.join("test2.md");
        let windsurf_symlink2 = windsurf_rules_path.join("test2.md");
        assert!(cursor_symlink2.exists(), "Cursor symlink 2 should exist");
        assert!(
            windsurf_symlink2.exists(),
            "Windsurf symlink 2 should exist"
        );

        // Delete the original file from .rules directory
        fs::remove_file(&test_file).unwrap();

        // Give daemon time to process the deletion
        thread::sleep(Duration::from_millis(300));

        // Verify symlinks were removed
        assert!(!cursor_symlink.exists(), "Cursor symlink should be removed");
        assert!(
            !windsurf_symlink.exists(),
            "Windsurf symlink should be removed"
        );

        // Verify second file symlinks still exist
        assert!(
            cursor_symlink2.exists(),
            "Cursor symlink 2 should still exist"
        );
        assert!(
            windsurf_symlink2.exists(),
            "Windsurf symlink 2 should still exist"
        );

        // Shutdown daemon
        shutdown_tx.send(()).unwrap();

        // Wait for daemon to finish
        let daemon_result = daemon_handle.join().unwrap();
        assert!(daemon_result.is_ok(), "Daemon should complete successfully");
    }

    #[test]
    #[serial_test::serial]
    fn test_single_instance_enforcement() {
        let dir1 = tempdir().unwrap();
        let dir2 = tempdir().unwrap();

        // Create .rules directories in both
        let rules_path1 = dir1.path().join(RULES_DIR);
        let rules_path2 = dir2.path().join(RULES_DIR);
        fs::create_dir(&rules_path1).unwrap();
        fs::create_dir(&rules_path2).unwrap();

        // Create channels for shutdown signals
        let (shutdown_tx1, shutdown_rx1) = mpsc::channel();
        let (_shutdown_tx2, shutdown_rx2) = mpsc::channel();

        // Start first daemon instance in dir1
        let daemon_dir1 = dir1.path().to_path_buf();
        let daemon_handle1 = thread::spawn(move || start_daemon(daemon_dir1, shutdown_rx1));

        // Give first daemon time to start and acquire system-wide lock
        thread::sleep(Duration::from_millis(200));

        // Try to start second daemon instance in dir2 - should fail due to system-wide lock
        let daemon_dir2 = dir2.path().to_path_buf();
        let daemon_handle2 = thread::spawn(move || start_daemon(daemon_dir2, shutdown_rx2));

        // Give second daemon time to try to start
        thread::sleep(Duration::from_millis(200));

        // Check if second daemon failed to start
        let daemon2_result = daemon_handle2.join().unwrap();
        assert!(
            daemon2_result.is_err(),
            "Second daemon should fail to start due to system-wide lock"
        );

        // Verify the error is about another instance running
        let error = daemon2_result.unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);

        // Shutdown first daemon
        let _ = shutdown_tx1.send(()); // Ignore send error if daemon already exited

        // Wait for first daemon to finish
        let daemon1_result = daemon_handle1.join().unwrap();
        assert!(
            daemon1_result.is_ok(),
            "First daemon should complete successfully"
        );

        // Now try to start a third daemon in dir2 - should succeed since first daemon stopped
        let (shutdown_tx3, shutdown_rx3) = mpsc::channel();
        let daemon_dir3 = dir2.path().to_path_buf();
        let daemon_handle3 = thread::spawn(move || start_daemon(daemon_dir3, shutdown_rx3));

        // Give third daemon time to start
        thread::sleep(Duration::from_millis(200));

        // Shutdown third daemon
        let _ = shutdown_tx3.send(()); // Ignore send error if daemon already exited

        // Wait for third daemon to finish
        let daemon3_result = daemon_handle3.join().unwrap();
        assert!(
            daemon3_result.is_ok(),
            "Third daemon should complete successfully"
        );
    }

    #[test]
    fn test_handle_rename_events() {
        let dir = tempdir().unwrap();

        // Create .rules directory
        let rules_path = dir.path().join(RULES_DIR);
        fs::create_dir(&rules_path).unwrap();

        // Create target directories
        let cursor_rules_path = dir.path().join(CURSOR_RULES_DIR);
        let windsurf_rules_path = dir.path().join(WINDSURF_RULES_DIR);
        fs::create_dir_all(&cursor_rules_path).unwrap();
        fs::create_dir_all(&windsurf_rules_path).unwrap();

        // Create an initial file and symlinks
        let old_file = rules_path.join("old_name.txt");
        fs::write(&old_file, "test content").unwrap();

        let old_cursor_symlink = cursor_rules_path.join("old_name.txt");
        let old_windsurf_symlink = windsurf_rules_path.join("old_name.txt");
        create_symlink_to_file(&old_file, &old_cursor_symlink).unwrap();
        create_symlink_to_file(&old_file, &old_windsurf_symlink).unwrap();

        // Verify initial symlinks exist
        assert!(
            old_cursor_symlink.exists(),
            "Initial cursor symlink should exist"
        );
        assert!(
            old_windsurf_symlink.exists(),
            "Initial windsurf symlink should exist"
        );

        // Simulate rename FROM event (old name being removed)
        let rules_path_canonical = rules_path.canonicalize().unwrap();
        let old_file_canonical = old_file.canonicalize().unwrap();
        let from_event = Event {
            kind: EventKind::Modify(ModifyKind::Name(RenameMode::From)),
            paths: vec![old_file_canonical],
            attrs: Default::default(),
        };

        handle_file_event(
            &from_event,
            &rules_path_canonical,
            &cursor_rules_path,
            &windsurf_rules_path,
        )
        .unwrap();

        // Verify old symlinks are removed
        assert!(
            !old_cursor_symlink.exists(),
            "Old cursor symlink should be removed"
        );
        assert!(
            !old_windsurf_symlink.exists(),
            "Old windsurf symlink should be removed"
        );

        // Rename the actual file to simulate the full rename
        let new_file = rules_path.join("new_name.txt");
        fs::rename(&old_file, &new_file).unwrap();

        // Simulate rename TO event (new name being created)
        let new_file_canonical = new_file.canonicalize().unwrap();
        let to_event = Event {
            kind: EventKind::Modify(ModifyKind::Name(RenameMode::To)),
            paths: vec![new_file_canonical],
            attrs: Default::default(),
        };

        handle_file_event(
            &to_event,
            &rules_path_canonical,
            &cursor_rules_path,
            &windsurf_rules_path,
        )
        .unwrap();

        // Verify new symlinks are created
        let new_cursor_symlink = cursor_rules_path.join("new_name.txt");
        let new_windsurf_symlink = windsurf_rules_path.join("new_name.txt");
        assert!(
            new_cursor_symlink.exists(),
            "New cursor symlink should be created"
        );
        assert!(
            new_windsurf_symlink.exists(),
            "New windsurf symlink should be created"
        );

        // Verify symlinks point to correct content
        let cursor_content = fs::read_to_string(&new_cursor_symlink).unwrap();
        let windsurf_content = fs::read_to_string(&new_windsurf_symlink).unwrap();
        assert_eq!(cursor_content, "test content");
        assert_eq!(windsurf_content, "test content");
    }
}
