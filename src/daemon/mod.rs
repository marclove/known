//! File watching daemon functionality for managing symlinks in rules directories.

pub mod config_handler;
pub mod events;
pub mod symlinks;
pub mod watchers;

use std::io;
use std::sync::mpsc;

use crate::config::load_config;
use crate::single_instance::SingleInstanceLock;

pub use config_handler::*;
pub use events::*;
pub use symlinks::*;
pub use watchers::*;

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

/// Starts the daemon with a configuration from a specific path (for testing)
#[cfg(test)]
pub fn start_daemon_from_config_file<P: AsRef<std::path::Path>>(
    shutdown_rx: mpsc::Receiver<()>,
    config_path: P,
) -> io::Result<()> {
    let config = crate::config::load_config_from_file(config_path)?;
    start_daemon_with_config_no_lock(shutdown_rx, config)
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

/// Prints the list of directories being watched.
fn print_watched_directories(watched_directories: &std::collections::HashSet<std::path::PathBuf>) {
    println!(
        "Watching {} directories for changes:",
        watched_directories.len()
    );
    for dir in watched_directories {
        println!("  - {}", dir.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::mpsc;
    use std::time::Duration;
    use tempfile::tempdir;

    #[test]
    fn test_daemon_with_no_directories() {
        let (_shutdown_tx, shutdown_rx) = mpsc::channel();
        let config = crate::config::Config::new();

        // Start daemon with empty config
        let result = start_daemon_with_test_config(shutdown_rx, config);

        // Should succeed but do nothing
        assert!(result.is_ok());
    }

    #[test]
    fn test_daemon_with_single_directory() {
        let temp_dir = tempdir().unwrap();
        let rules_path = temp_dir.path().join(crate::constants::RULES_DIR);
        fs::create_dir_all(&rules_path).unwrap();

        // Create a test file
        fs::write(rules_path.join("test.md"), "test content").unwrap();

        let mut config = crate::config::Config::new();
        config.add_directory(temp_dir.path().to_path_buf());

        let (shutdown_tx, shutdown_rx) = mpsc::channel();

        // Start daemon in a separate thread
        let config_clone = config.clone();
        let handle =
            std::thread::spawn(move || start_daemon_with_test_config(shutdown_rx, config_clone));

        // Give daemon time to start
        std::thread::sleep(Duration::from_millis(100));

        // Signal shutdown
        shutdown_tx.send(()).unwrap();

        // Wait for daemon to finish
        let result = handle.join().unwrap();
        assert!(result.is_ok());
    }

    #[test]
    fn test_daemon_with_nonexistent_directory() {
        let temp_dir = tempdir().unwrap();
        let nonexistent_path = temp_dir.path().join("nonexistent");

        let mut config = crate::config::Config::new();
        config.add_directory(nonexistent_path);

        let (_shutdown_tx, shutdown_rx) = mpsc::channel();

        // Start daemon with nonexistent directory
        let result = start_daemon_with_test_config(shutdown_rx, config);

        // May return an error if watcher setup fails, which is acceptable behavior
        // The important thing is that it doesn't panic
        if let Err(e) = result {
            // Log the error but don't fail the test - this is expected behavior
            println!("Expected error for nonexistent directory: {}", e);
        }
        // Test that it doesn't panic - reaching this point means success
    }

    #[test]
    fn test_daemon_print_watched_directories() {
        let temp_dir1 = tempdir().unwrap();
        let temp_dir2 = tempdir().unwrap();

        let mut watched_directories = std::collections::HashSet::new();
        watched_directories.insert(temp_dir1.path().to_path_buf());
        watched_directories.insert(temp_dir2.path().to_path_buf());

        // This should not panic and should print the directories
        print_watched_directories(&watched_directories);
    }

    #[test]
    fn test_daemon_from_config_file() {
        let temp_dir = tempdir().unwrap();
        let rules_path = temp_dir.path().join(crate::constants::RULES_DIR);
        fs::create_dir_all(&rules_path).unwrap();

        // Create config file
        let config_dir = tempdir().unwrap();
        let config_path = config_dir.path().join("config.json");

        let mut config = crate::config::Config::new();
        config.add_directory(temp_dir.path().to_path_buf());
        crate::config::save_config_to_file(&config, &config_path).unwrap();

        let (shutdown_tx, shutdown_rx) = mpsc::channel();

        // Start daemon from config file
        let config_path_clone = config_path.clone();
        let handle = std::thread::spawn(move || {
            start_daemon_from_config_file(shutdown_rx, config_path_clone)
        });

        // Give daemon time to start
        std::thread::sleep(Duration::from_millis(100));

        // Signal shutdown
        shutdown_tx.send(()).unwrap();

        // Wait for daemon to finish
        let result = handle.join().unwrap();
        assert!(result.is_ok());
    }

    #[test]
    fn test_daemon_from_invalid_config_file() {
        let temp_dir = tempdir().unwrap();
        let invalid_config_path = temp_dir.path().join("invalid_config.json");

        // Create invalid config file
        fs::write(&invalid_config_path, "invalid json").unwrap();

        let (_shutdown_tx, shutdown_rx) = mpsc::channel();

        // Start daemon from invalid config file
        let result = start_daemon_from_config_file(shutdown_rx, invalid_config_path);

        // Should return error for invalid config
        assert!(result.is_err());
    }

    #[test]
    fn test_daemon_with_multiple_directories() {
        let temp_dir1 = tempdir().unwrap();
        let temp_dir2 = tempdir().unwrap();
        let temp_dir3 = tempdir().unwrap();

        // Create .rules directories
        let rules_path1 = temp_dir1.path().join(crate::constants::RULES_DIR);
        let rules_path2 = temp_dir2.path().join(crate::constants::RULES_DIR);
        let rules_path3 = temp_dir3.path().join(crate::constants::RULES_DIR);
        fs::create_dir_all(&rules_path1).unwrap();
        fs::create_dir_all(&rules_path2).unwrap();
        fs::create_dir_all(&rules_path3).unwrap();

        // Create test files
        fs::write(rules_path1.join("test1.md"), "content1").unwrap();
        fs::write(rules_path2.join("test2.md"), "content2").unwrap();
        fs::write(rules_path3.join("test3.md"), "content3").unwrap();

        let mut config = crate::config::Config::new();
        config.add_directory(temp_dir1.path().to_path_buf());
        config.add_directory(temp_dir2.path().to_path_buf());
        config.add_directory(temp_dir3.path().to_path_buf());

        let (shutdown_tx, shutdown_rx) = mpsc::channel();

        // Start daemon in a separate thread
        let config_clone = config.clone();
        let handle =
            std::thread::spawn(move || start_daemon_with_test_config(shutdown_rx, config_clone));

        // Give daemon time to start
        std::thread::sleep(Duration::from_millis(200));

        // Signal shutdown
        shutdown_tx.send(()).unwrap();

        // Wait for daemon to finish
        let result = handle.join().unwrap();
        assert!(result.is_ok());
    }
}
