//! Single instance enforcement functionality for daemons.
//!
//! This module provides functionality to ensure only one instance of a daemon
//! process can run at a time using PID files and file locking.

mod lock;
mod path;
mod process;
mod stop;

pub use lock::SingleInstanceLock;
pub use stop::stop_daemon;

#[cfg(test)]
mod tests {
    use super::lock::SingleInstanceLock;
    use super::process::is_process_running;
    use super::stop::stop_daemon_with_test_path;
    use std::io;
    use std::thread;
    use std::time::Duration;
    use tempfile::tempdir;

    #[test]
    fn test_single_instance_lock_acquisition() {
        let test_dir = tempdir().unwrap();
        let test_lock_path = test_dir.path().join("test_lock.pid");

        // First instance should successfully acquire lock
        let lock1 = SingleInstanceLock::acquire_with_test_path(&test_lock_path).unwrap();

        // Verify PID file was created
        assert!(test_lock_path.exists(), "PID file should be created");

        // Verify PID file contains current process ID
        let contents = std::fs::read_to_string(&test_lock_path).unwrap();
        let stored_pid: u32 = contents.trim().parse().unwrap();
        assert_eq!(stored_pid, std::process::id());

        // Second instance should fail to acquire lock
        let lock2_result = SingleInstanceLock::acquire_with_test_path(&test_lock_path);
        assert!(
            lock2_result.is_err(),
            "Second instance should fail to acquire lock"
        );

        // Verify error message indicates another instance is running
        let error = lock2_result.unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        assert!(error.to_string().contains("already running"));

        // Drop first lock
        drop(lock1);

        // Verify PID file was removed
        assert!(
            !test_lock_path.exists(),
            "PID file should be removed after drop"
        );

        // Third instance should now succeed
        let lock3 = SingleInstanceLock::acquire_with_test_path(&test_lock_path).unwrap();
        assert!(test_lock_path.exists(), "PID file should be created again");

        drop(lock3);
    }

    #[test]
    fn test_stale_pid_file_handling() {
        let test_dir = tempdir().unwrap();
        let test_lock_path = test_dir.path().join("test_stale.pid");

        // Create a stale PID file with a non-existent PID
        let fake_pid = 999999; // Very unlikely to be a real PID
        std::fs::write(&test_lock_path, format!("{}\n", fake_pid)).unwrap();

        // Should still be able to acquire lock despite stale PID file
        let lock = SingleInstanceLock::acquire_with_test_path(&test_lock_path).unwrap();

        // Verify the PID file now contains the current process ID
        let contents = std::fs::read_to_string(&test_lock_path).unwrap();
        let stored_pid: u32 = contents.trim().parse().unwrap();
        assert_eq!(stored_pid, std::process::id());

        drop(lock);
    }

    #[test]
    fn test_concurrent_lock_acquisition() {
        let test_dir = tempdir().unwrap();
        let test_lock_path = test_dir.path().join("test_concurrent.pid");

        // Spawn multiple threads trying to acquire lock simultaneously
        let handles: Vec<_> = (0..5)
            .map(|i| {
                let lock_path_clone = test_lock_path.clone();
                thread::spawn(move || {
                    thread::sleep(Duration::from_millis(i * 10)); // Slight delay variation
                    SingleInstanceLock::acquire_with_test_path(lock_path_clone)
                })
            })
            .collect();

        let mut results = Vec::new();
        for handle in handles {
            results.push(handle.join().unwrap());
        }

        // Exactly one thread should succeed
        let successful_count = results.iter().filter(|r| r.is_ok()).count();
        assert_eq!(
            successful_count, 1,
            "Exactly one thread should acquire the lock"
        );

        // All others should fail with AlreadyExists error
        let failed_count = results.iter().filter(|r| r.is_err()).count();
        assert_eq!(
            failed_count, 4,
            "Four threads should fail to acquire the lock"
        );

        for result in &results {
            if let Err(e) = result {
                assert_eq!(e.kind(), io::ErrorKind::AlreadyExists);
            }
        }

        // Clean up the successful lock
        for result in results {
            if let Ok(lock) = result {
                drop(lock);
                break;
            }
        }
    }

    #[test]
    fn test_system_wide_single_instance_enforcement() {
        // Test that the system-wide lock works by using a test-specific lock path
        // Only one instance should be allowed system-wide

        let test_dir = tempdir().unwrap();
        let test_lock_path = test_dir.path().join("test_system_wide.pid");

        // First, test that we can acquire a system-wide lock
        let lock1 = SingleInstanceLock::acquire_with_test_path(&test_lock_path).unwrap();

        // Attempting to acquire another system-wide lock should fail
        let lock2_result = SingleInstanceLock::acquire_with_test_path(&test_lock_path);
        assert!(lock2_result.is_err(), "Second system-wide lock should fail");

        // Verify error message indicates another instance is running
        let error = lock2_result.unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);

        // Drop first lock
        drop(lock1);

        // Now it should work again
        let lock3 = SingleInstanceLock::acquire_with_test_path(&test_lock_path).unwrap();
        drop(lock3);
    }

    #[test]
    fn test_stop_daemon_no_pid_file() {
        // Test stopping daemon when no PID file exists
        let test_dir = tempdir().unwrap();
        let test_lock_path = test_dir.path().join("nonexistent.pid");

        // Use the test-specific function to avoid affecting real daemon processes
        assert!(!test_lock_path.exists(), "Test PID file should not exist");

        let result = stop_daemon_with_test_path(&test_lock_path);
        assert!(result.is_err(), "Should fail when PID file doesn't exist");

        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
        assert!(err.to_string().contains("No daemon is currently running"));
    }

    #[test]
    fn test_stop_daemon_invalid_pid_file() {
        let test_dir = tempdir().unwrap();
        let test_lock_path = test_dir.path().join("invalid.pid");

        // Create a PID file with invalid content
        std::fs::write(&test_lock_path, "not_a_number\n").unwrap();

        // Test that the function properly handles invalid PID content
        let result = stop_daemon_with_test_path(&test_lock_path);
        assert!(
            result.is_err(),
            "Should fail when PID file contains invalid data"
        );

        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("Invalid PID in lock file"));

        // Test with another invalid content
        std::fs::write(&test_lock_path, "123a\n").unwrap();
        let result = stop_daemon_with_test_path(&test_lock_path);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn test_stop_daemon_empty_pid_file() {
        let test_dir = tempdir().unwrap();
        let test_lock_path = test_dir.path().join("empty.pid");

        // Create an empty PID file
        std::fs::write(&test_lock_path, "").unwrap();

        // Test that the function properly handles empty PID file
        let result = stop_daemon_with_test_path(&test_lock_path);
        assert!(result.is_err(), "Should fail when PID file is empty");

        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("PID file is empty"));

        // Test with a file containing only whitespace
        std::fs::write(&test_lock_path, "   \n\t").unwrap();
        let result = stop_daemon_with_test_path(&test_lock_path);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn test_stop_daemon_stale_pid_file() {
        let test_dir = tempdir().unwrap();
        let test_lock_path = test_dir.path().join("stale.pid");

        // Create a PID file with a non-existent PID
        let fake_pid = 999999; // Very unlikely to be a real PID
        std::fs::write(&test_lock_path, format!("{}\n", fake_pid)).unwrap();

        // Test that is_process_running correctly identifies non-existent processes
        assert!(
            !is_process_running(fake_pid),
            "Fake PID should not be running"
        );

        assert!(
            test_lock_path.exists(),
            "Test file should exist before cleanup"
        );

        // Test that stop_daemon properly handles stale PID files
        let result = stop_daemon_with_test_path(&test_lock_path);
        assert!(result.is_err(), "Should fail when PID is not running");

        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
        assert!(err.to_string().contains("is not running"));

        // Verify the stale PID file was removed
        assert!(!test_lock_path.exists(), "Stale PID file should be removed");
    }

    #[test]
    fn test_acquire_lock_permission_denied() {
        let temp_dir = tempdir().unwrap();
        let readonly_dir = temp_dir.path().join("readonly");
        std::fs::create_dir(&readonly_dir).unwrap();

        if cfg!(unix) {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o555);
            std::fs::set_permissions(&readonly_dir, perms).unwrap();
        } else {
            let mut perms = std::fs::metadata(&readonly_dir).unwrap().permissions();
            perms.set_readonly(true);
            std::fs::set_permissions(&readonly_dir, perms).unwrap();
        }

        let lock_path = readonly_dir.join("test.pid");
        let result = SingleInstanceLock::acquire_with_test_path(&lock_path);

        if result.is_err() {
            assert_eq!(result.unwrap_err().kind(), io::ErrorKind::PermissionDenied);
        }
    }

    #[test]
    fn test_corrupted_pid_file_handling() {
        let test_dir = tempdir().unwrap();
        let test_lock_path = test_dir.path().join("corrupted.pid");

        // Create a PID file with binary data that can't be read as valid UTF-8
        std::fs::write(&test_lock_path, vec![0xFF, 0xFE, 0xFD]).unwrap();

        // Should handle corrupted file gracefully by overwriting it
        let lock = SingleInstanceLock::acquire_with_test_path(&test_lock_path);

        // Should either succeed (by overwriting corrupted data) or fail gracefully
        match lock {
            Ok(lock) => {
                // Verify that it wrote a valid PID
                let contents = std::fs::read_to_string(&test_lock_path).unwrap();
                let stored_pid: u32 = contents.trim().parse().unwrap();
                assert_eq!(stored_pid, std::process::id());
                drop(lock);
            }
            Err(e) => {
                // Error should be related to file handling, not a panic
                assert!(
                    e.kind() == io::ErrorKind::InvalidData
                        || e.kind() == io::ErrorKind::Other
                        || e.kind() == io::ErrorKind::PermissionDenied
                );
            }
        }
    }

    #[test]
    fn test_pid_file_with_extra_content() {
        let test_dir = tempdir().unwrap();
        let test_lock_path = test_dir.path().join("extra_content.pid");

        // First, acquire a lock successfully
        let _lock = SingleInstanceLock::acquire_with_test_path(&test_lock_path).unwrap();

        // Manually modify the PID file to add extra content after the PID
        let current_pid = std::process::id();
        std::fs::write(
            &test_lock_path,
            format!("{}\nextra content\nmore stuff", current_pid),
        )
        .unwrap();

        // Second lock acquisition should detect this as a running process
        let result = SingleInstanceLock::acquire_with_test_path(&test_lock_path);
        assert!(
            result.is_err(),
            "Should fail when PID file indicates running process"
        );

        let error = result.unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        assert!(error.to_string().contains("already running"));
    }

    #[test]
    fn test_pid_file_parsing_edge_cases() {
        let test_dir = tempdir().unwrap();

        // Test with PID file containing only whitespace and newlines
        let test_lock_path = test_dir.path().join("whitespace.pid");
        std::fs::write(&test_lock_path, "  \n\t  \n  ").unwrap();

        // Should be able to acquire lock when file is effectively empty
        let lock = SingleInstanceLock::acquire_with_test_path(&test_lock_path).unwrap();
        drop(lock);

        // Test with PID file containing unparseable content
        std::fs::write(&test_lock_path, "not_a_number").unwrap();

        // Should be able to acquire lock when PID can't be parsed
        let lock = SingleInstanceLock::acquire_with_test_path(&test_lock_path).unwrap();
        drop(lock);

        // Test with PID file containing negative number
        std::fs::write(&test_lock_path, "-123").unwrap();

        // Should be able to acquire lock when PID is invalid
        let lock = SingleInstanceLock::acquire_with_test_path(&test_lock_path).unwrap();
        drop(lock);
    }
}
