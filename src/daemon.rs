//! File watching daemon functionality for managing symlinks in rules directories.

use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::fs;
use std::io;
use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

use crate::symlinks::create_symlink_to_file;

/// The directory name for rules files
const RULES_DIR: &str = ".rules";

/// The directory name for cursor rules files
const CURSOR_RULES_DIR: &str = ".cursor/rules";

/// The directory name for windsurf rules files
const WINDSURF_RULES_DIR: &str = ".windsurf/rules";

/// Starts a daemon that watches the .rules directory for changes and maintains
/// synchronized symlinks in .cursor/rules and .windsurf/rules directories.
///
/// This function creates a file watcher that monitors the .rules directory for
/// file additions, modifications, and deletions. When changes are detected,
/// it automatically updates the corresponding symlinks in the .cursor/rules
/// and .windsurf/rules directories to keep them in sync.
///
/// # Behavior
///
/// - Watches the .rules directory recursively for file system events
/// - Creates symlinks in .cursor/rules and .windsurf/rules for each file in .rules
/// - Removes symlinks when files are deleted from .rules
/// - Runs indefinitely until the receiver channel is closed
/// - Prints status messages to stdout for user feedback
///
/// # Arguments
///
/// * `dir` - The directory path containing the .rules directory to watch
/// * `shutdown_rx` - A receiver channel that signals when to stop the daemon
///
/// # Errors
///
/// Returns an error if:
/// - The .rules directory doesn't exist
/// - Watcher creation fails
/// - File system operations fail
/// - Directory creation fails
///
pub fn start_daemon<P: AsRef<Path>>(dir: P, shutdown_rx: mpsc::Receiver<()>) -> io::Result<()> {
    let dir = dir.as_ref();
    let rules_path = dir.join(RULES_DIR);

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
            EventKind::Create(_) | EventKind::Modify(_) => {
                // Create or update symlinks
                if path.is_file() {
                    create_symlink_to_file(path, &cursor_target)?;
                    create_symlink_to_file(path, &windsurf_target)?;
                    println!("Created symlinks for {}", file_name.to_string_lossy());
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
}