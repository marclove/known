//! File system event handling for the daemon.

use super::config_handler::handle_config_file_change_internal;
use super::file_event::handle_file_event;
use super::watchers::WatcherSetup;
use crate::daemon::config_event::is_config_file_event;
use std::io;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

/// Runs the main daemon event loop.
pub fn run_daemon_event_loop(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::RULES_DIR;
    use crate::daemon::watchers;
    use std::collections::HashMap;
    use std::fs;
    use std::sync::mpsc;
    use tempfile::tempdir;

    #[test]
    fn test_daemon_event_loop_with_shutdown() {
        // Test event loop with immediate shutdown
        let dir = tempdir().unwrap();
        let rules_path = dir.path().join(RULES_DIR);
        fs::create_dir(&rules_path).unwrap();

        let mut config = crate::config::Config::new();
        config.add_directory(dir.path().to_path_buf());

        let mut watched_directories = config.get_watched_directories().clone();

        // Create a mock watcher setup
        let (_tx, rx) = mpsc::channel();
        let mut rules_paths = HashMap::new();
        rules_paths.insert(rules_path.canonicalize().unwrap(), dir.path().to_path_buf());

        let watcher_setup = watchers::WatcherSetup {
            watchers: Vec::new(),
            rules_paths,
            event_receiver: rx,
            config_file_path: std::env::temp_dir().join("test_config.json"),
        };

        // Create shutdown channel and immediately send shutdown signal
        let (shutdown_tx, shutdown_rx) = mpsc::channel();
        shutdown_tx.send(()).unwrap();

        // Run daemon event loop - should exit immediately
        let result = run_daemon_event_loop(
            shutdown_rx,
            &mut config,
            &mut watched_directories,
            watcher_setup,
        );

        assert!(result.is_ok());
    }
}
