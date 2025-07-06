//! File system event handling for the daemon.

use notify::{
    event::{ModifyKind, RenameMode},
    Event, EventKind,
};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use crate::constants::{CURSOR_RULES_DIR, WINDSURF_RULES_DIR};
use crate::symlinks::create_symlink_to_file;

use super::config_handler::handle_config_file_change_internal;
use super::watchers::WatcherSetup;

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
pub fn handle_file_event(event: &Event, rules_paths: &HashMap<PathBuf, PathBuf>) -> io::Result<()> {
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

/// Checks if the file event is related to the configuration file
pub fn is_config_file_event(event: &Event, config_file_path: &Path) -> bool {
    event.paths.iter().any(|path| {
        path.file_name() == config_file_path.file_name()
            && path.parent() == config_file_path.parent()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::RULES_DIR;
    use crate::daemon::watchers;
    use std::collections::HashMap;
    use tempfile::tempdir;

    #[test]
    fn test_handle_file_event_with_multiple_directories() {
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

    // Additional event handling tests moved from daemon.rs
    #[test]
    fn test_handle_file_event_rename_operations() {
        // Test rename operations (RenameMode::From and RenameMode::To)
        let dir = tempdir().unwrap();
        let rules_path = dir.path().join(RULES_DIR);
        fs::create_dir(&rules_path).unwrap();

        // Create target directories
        let cursor_rules_path = dir.path().join(CURSOR_RULES_DIR);
        let windsurf_rules_path = dir.path().join(WINDSURF_RULES_DIR);
        fs::create_dir_all(&cursor_rules_path).unwrap();
        fs::create_dir_all(&windsurf_rules_path).unwrap();

        // Create initial file and symlinks
        let test_file = rules_path.join("test.md");
        fs::write(&test_file, "content").unwrap();
        create_symlink_to_file(&test_file, &cursor_rules_path.join("test.md")).unwrap();
        create_symlink_to_file(&test_file, &windsurf_rules_path.join("test.md")).unwrap();

        // Verify initial symlinks exist
        assert!(cursor_rules_path.join("test.md").exists());
        assert!(windsurf_rules_path.join("test.md").exists());

        // Create rules paths map
        let mut rules_paths = HashMap::new();
        rules_paths.insert(rules_path.canonicalize().unwrap(), dir.path().to_path_buf());

        // Test RenameMode::From - should remove old symlinks
        let rename_from_event = Event {
            kind: EventKind::Modify(ModifyKind::Name(RenameMode::From)),
            paths: vec![test_file.canonicalize().unwrap()],
            attrs: Default::default(),
        };

        handle_file_event(&rename_from_event, &rules_paths).unwrap();

        // Verify symlinks were removed
        assert!(!cursor_rules_path.join("test.md").exists());
        assert!(!windsurf_rules_path.join("test.md").exists());

        // Test RenameMode::To - should create new symlinks
        let new_file = rules_path.join("renamed.md");
        fs::write(&new_file, "content").unwrap();

        let rename_to_event = Event {
            kind: EventKind::Modify(ModifyKind::Name(RenameMode::To)),
            paths: vec![new_file.canonicalize().unwrap()],
            attrs: Default::default(),
        };

        handle_file_event(&rename_to_event, &rules_paths).unwrap();

        // Verify new symlinks were created
        assert!(cursor_rules_path.join("renamed.md").exists());
        assert!(windsurf_rules_path.join("renamed.md").exists());
    }

    #[test]
    fn test_handle_file_event_modify_operations() {
        // Test various modify operations
        let dir = tempdir().unwrap();
        let rules_path = dir.path().join(RULES_DIR);
        fs::create_dir(&rules_path).unwrap();

        // Create target directories
        let cursor_rules_path = dir.path().join(CURSOR_RULES_DIR);
        let windsurf_rules_path = dir.path().join(WINDSURF_RULES_DIR);
        fs::create_dir_all(&cursor_rules_path).unwrap();
        fs::create_dir_all(&windsurf_rules_path).unwrap();

        // Create test file
        let test_file = rules_path.join("test.md");
        fs::write(&test_file, "original content").unwrap();

        // Create rules paths map
        let mut rules_paths = HashMap::new();
        rules_paths.insert(rules_path.canonicalize().unwrap(), dir.path().to_path_buf());

        // Test content modification - should create/update symlinks
        let modify_event = Event {
            kind: EventKind::Modify(ModifyKind::Data(notify::event::DataChange::Content)),
            paths: vec![test_file.canonicalize().unwrap()],
            attrs: Default::default(),
        };

        handle_file_event(&modify_event, &rules_paths).unwrap();

        // Verify symlinks were created/updated
        assert!(cursor_rules_path.join("test.md").exists());
        assert!(windsurf_rules_path.join("test.md").exists());

        // Test metadata modification - should also update symlinks
        let metadata_event = Event {
            kind: EventKind::Modify(ModifyKind::Metadata(notify::event::MetadataKind::Permissions)),
            paths: vec![test_file.canonicalize().unwrap()],
            attrs: Default::default(),
        };

        handle_file_event(&metadata_event, &rules_paths).unwrap();

        // Verify symlinks still exist
        assert!(cursor_rules_path.join("test.md").exists());
        assert!(windsurf_rules_path.join("test.md").exists());

        // Test modification when file doesn't exist (should be ignored)
        let nonexistent_file = rules_path.join("nonexistent.md");
        let modify_nonexistent_event = Event {
            kind: EventKind::Modify(ModifyKind::Data(notify::event::DataChange::Content)),
            paths: vec![nonexistent_file.clone()],
            attrs: Default::default(),
        };

        // This should not create symlinks since the file doesn't exist
        handle_file_event(&modify_nonexistent_event, &rules_paths).unwrap();
        assert!(!cursor_rules_path.join("nonexistent.md").exists());
        assert!(!windsurf_rules_path.join("nonexistent.md").exists());
    }

    #[test]
    fn test_handle_file_event_remove_operations() {
        // Test file removal
        let dir = tempdir().unwrap();
        let rules_path = dir.path().join(RULES_DIR);
        fs::create_dir(&rules_path).unwrap();

        // Create target directories
        let cursor_rules_path = dir.path().join(CURSOR_RULES_DIR);
        let windsurf_rules_path = dir.path().join(WINDSURF_RULES_DIR);
        fs::create_dir_all(&cursor_rules_path).unwrap();
        fs::create_dir_all(&windsurf_rules_path).unwrap();

        // Create test file and symlinks
        let test_file = rules_path.join("test.md");
        fs::write(&test_file, "content").unwrap();
        create_symlink_to_file(&test_file, &cursor_rules_path.join("test.md")).unwrap();
        create_symlink_to_file(&test_file, &windsurf_rules_path.join("test.md")).unwrap();

        // Verify symlinks exist initially
        assert!(cursor_rules_path.join("test.md").exists());
        assert!(windsurf_rules_path.join("test.md").exists());

        // Create rules paths map
        let mut rules_paths = HashMap::new();
        rules_paths.insert(rules_path.canonicalize().unwrap(), dir.path().to_path_buf());

        // Test file removal - remove event should clean up symlinks
        let remove_event = Event {
            kind: EventKind::Remove(notify::event::RemoveKind::File),
            paths: vec![test_file.canonicalize().unwrap()],
            attrs: Default::default(),
        };

        handle_file_event(&remove_event, &rules_paths).unwrap();

        // Verify symlinks were removed
        assert!(!cursor_rules_path.join("test.md").exists());
        assert!(!windsurf_rules_path.join("test.md").exists());

        // Test removal of non-existent symlinks (should not error)
        let nonexistent_file = rules_path.join("nonexistent.md");
        let remove_nonexistent_event = Event {
            kind: EventKind::Remove(notify::event::RemoveKind::File),
            paths: vec![nonexistent_file],
            attrs: Default::default(),
        };

        // This should not error even if symlinks don't exist
        let result = handle_file_event(&remove_nonexistent_event, &rules_paths);
        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_file_event_ignore_non_watched_directories() {
        // Test that events outside watched directories are ignored
        let watched_dir = tempdir().unwrap();
        let unwatched_dir = tempdir().unwrap();

        let rules_path = watched_dir.path().join(RULES_DIR);
        fs::create_dir(&rules_path).unwrap();

        // Create rules paths map with only watched directory
        let mut rules_paths = HashMap::new();
        rules_paths.insert(rules_path.canonicalize().unwrap(), watched_dir.path().to_path_buf());

        // Create event for file in unwatched directory
        let unwatched_file = unwatched_dir.path().join("test.md");
        fs::write(&unwatched_file, "content").unwrap();

        let event = Event {
            kind: EventKind::Create(notify::event::CreateKind::File),
            paths: vec![unwatched_file],
            attrs: Default::default(),
        };

        // This should not create any symlinks and should not error
        let result = handle_file_event(&event, &rules_paths);
        assert!(result.is_ok());

        // Verify no symlinks were created in watched directory
        let cursor_rules_path = watched_dir.path().join(CURSOR_RULES_DIR);
        let windsurf_rules_path = watched_dir.path().join(WINDSURF_RULES_DIR);
        
        if cursor_rules_path.exists() {
            assert!(!cursor_rules_path.join("test.md").exists());
        }
        if windsurf_rules_path.exists() {
            assert!(!windsurf_rules_path.join("test.md").exists());
        }
    }

    #[test]
    fn test_handle_file_event_without_filename() {
        // Test event handling when file path has no filename
        let dir = tempdir().unwrap();
        let rules_path = dir.path().join(RULES_DIR);
        fs::create_dir(&rules_path).unwrap();

        let mut rules_paths = HashMap::new();
        rules_paths.insert(rules_path.canonicalize().unwrap(), dir.path().to_path_buf());

        // Create an event with a directory path (no filename)
        let event = Event {
            kind: EventKind::Create(notify::event::CreateKind::File),
            paths: vec![rules_path.clone()],
            attrs: Default::default(),
        };

        // This should not panic and should handle gracefully
        let result = handle_file_event(&event, &rules_paths);
        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_file_event_unsupported_event_types() {
        // Test handling of unsupported event types
        let dir = tempdir().unwrap();
        let rules_path = dir.path().join(RULES_DIR);
        fs::create_dir(&rules_path).unwrap();

        let mut rules_paths = HashMap::new();
        rules_paths.insert(rules_path.canonicalize().unwrap(), dir.path().to_path_buf());

        let test_file = rules_path.join("test.md");
        fs::write(&test_file, "content").unwrap();

        // Test "Other" event type
        let other_event = Event {
            kind: EventKind::Other,
            paths: vec![test_file.clone()],
            attrs: Default::default(),
        };

        let result = handle_file_event(&other_event, &rules_paths);
        assert!(result.is_ok());

        // Test "Access" event type
        let access_event = Event {
            kind: EventKind::Access(notify::event::AccessKind::Open(notify::event::AccessMode::Read)),
            paths: vec![test_file.clone()],
            attrs: Default::default(),
        };

        let result = handle_file_event(&access_event, &rules_paths);
        assert!(result.is_ok());
    }

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

    #[test]
    fn test_is_config_file_event_matching() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("config.json");
        fs::write(&config_path, "{}").unwrap();

        // Test matching config file event
        let matching_event = Event {
            kind: EventKind::Modify(ModifyKind::Data(notify::event::DataChange::Content)),
            paths: vec![config_path.clone()],
            attrs: Default::default(),
        };

        assert!(is_config_file_event(&matching_event, &config_path));

        // Test non-matching event
        let other_file = temp_dir.path().join("other.json");
        fs::write(&other_file, "{}").unwrap();
        
        let non_matching_event = Event {
            kind: EventKind::Modify(ModifyKind::Data(notify::event::DataChange::Content)),
            paths: vec![other_file],
            attrs: Default::default(),
        };

        assert!(!is_config_file_event(&non_matching_event, &config_path));
    }

    #[test]
    fn test_is_config_file_event_different_directory() {
        let temp_dir1 = tempdir().unwrap();
        let temp_dir2 = tempdir().unwrap();
        
        let config_path1 = temp_dir1.path().join("config.json");
        let config_path2 = temp_dir2.path().join("config.json");
        
        fs::write(&config_path1, "{}").unwrap();
        fs::write(&config_path2, "{}").unwrap();

        // Test same filename but different directory
        let event = Event {
            kind: EventKind::Modify(ModifyKind::Data(notify::event::DataChange::Content)),
            paths: vec![config_path2.clone()],
            attrs: Default::default(),
        };

        assert!(!is_config_file_event(&event, &config_path1));
        assert!(is_config_file_event(&event, &config_path2));
    }
}