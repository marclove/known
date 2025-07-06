//! Single instance enforcement functionality for daemons.
//!
//! This module provides functionality to ensure only one instance of a daemon
//! process can run at a time using PID files and file locking.

use directories::ProjectDirs;
use nix::fcntl::{Flock, FlockArg};
use nix::sys::signal::kill;
use nix::unistd::Pid;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;

/// The name of the PID file used for single instance enforcement
const PID_FILE_NAME: &str = "known_daemon.pid";

/// Represents a single instance lock using a PID file
#[derive(Debug)]
pub struct SingleInstanceLock {
    /// The locked file handle
    _flock: Flock<File>,
    /// Path to the PID file
    pid_file_path: std::path::PathBuf,
}

/// Gets the system-wide lock file path using the directories crate
/// 
/// This function returns the path to the PID file in the application's
/// data directory, which is platform-specific and system-wide.
/// 
/// # Returns
/// 
/// Returns `Ok(PathBuf)` with the path to the PID file, or an error
/// if the application directories cannot be determined.
/// 
/// # Errors
/// 
/// Returns an error if the platform doesn't support application directories
/// or if directory creation fails.
fn get_system_wide_lock_path() -> io::Result<std::path::PathBuf> {
    let project_dirs = ProjectDirs::from("", "", "known")
        .ok_or_else(|| io::Error::new(
            io::ErrorKind::Other,
            "Unable to determine application directories for this platform"
        ))?;
    
    let data_dir = project_dirs.data_dir();
    
    // Create the data directory if it doesn't exist
    std::fs::create_dir_all(data_dir)?;
    
    Ok(data_dir.join(PID_FILE_NAME))
}

impl SingleInstanceLock {
    /// Attempts to acquire a system-wide single instance lock for the daemon process.
    ///
    /// This function creates or opens a PID file in the system's application data
    /// directory and attempts to acquire an exclusive lock on it. This ensures
    /// only one instance of the daemon can run system-wide, regardless of which
    /// directory it's launched from.
    ///
    /// # Returns
    ///
    /// Returns `Ok(SingleInstanceLock)` if the lock was successfully acquired,
    /// or an error if another instance is already running or if file operations fail.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Another instance of the daemon is already running system-wide
    /// - File operations fail (permissions, disk space, etc.)
    /// - The PID file contains an invalid process ID
    /// - Unable to determine application directories for this platform
    ///
    pub fn acquire() -> io::Result<Self> {
        let pid_file_path = get_system_wide_lock_path()?;
        Self::acquire_with_path(pid_file_path)
    }
    
    /// Acquires a lock with a custom file path (for testing)
    #[cfg(test)]
    pub fn acquire_with_test_path<P: AsRef<std::path::Path>>(pid_file_path: P) -> io::Result<Self> {
        Self::acquire_with_path(pid_file_path.as_ref().to_path_buf())
    }
    
    /// Internal function that handles the actual lock acquisition logic
    fn acquire_with_path(pid_file_path: std::path::PathBuf) -> io::Result<Self> {
        // Open or create the PID file
        let mut file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(&pid_file_path)?;

        // Read existing PID if any to check for stale processes
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        
        if !contents.trim().is_empty() {
            // Check if the existing PID is still running
            if let Ok(existing_pid) = contents.trim().parse::<i32>() {
                if is_process_running(existing_pid) {
                    return Err(io::Error::new(
                        io::ErrorKind::AlreadyExists,
                        format!("Another instance is already running with PID {}", existing_pid),
                    ));
                }
            }
        }

        // Try to acquire exclusive lock on the PID file
        match Flock::lock(file, FlockArg::LockExclusiveNonblock) {
            Ok(flock) => {
                // Successfully acquired lock, write current PID
                let current_pid = std::process::id();
                
                // Access the file through the flock to write the PID
                let mut file_copy = flock.try_clone()?;
                file_copy.seek(SeekFrom::Start(0))?;
                file_copy.set_len(0)?; // Truncate the file
                writeln!(file_copy, "{}", current_pid)?;
                file_copy.sync_all()?;
                
                Ok(SingleInstanceLock {
                    _flock: flock,
                    pid_file_path,
                })
            }
            Err((_, _)) => {
                // Failed to acquire lock, another instance is running
                Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "Another instance of the daemon is already running",
                ))
            }
        }
    }
    
    /// Returns the path to the PID file
    pub fn pid_file_path(&self) -> &Path {
        &self.pid_file_path
    }
}

impl Drop for SingleInstanceLock {
    /// Automatically releases the lock and cleans up the PID file when dropped
    fn drop(&mut self) {
        // The file lock is automatically released when the file is closed
        // Remove the PID file
        if let Err(e) = std::fs::remove_file(&self.pid_file_path) {
            eprintln!("Warning: Failed to remove PID file: {}", e);
        }
    }
}

/// Checks if a process with the given PID is currently running
fn is_process_running(pid: i32) -> bool {
    match kill(Pid::from_raw(pid), None) {
        Ok(()) => true,  // Process exists
        Err(_) => false, // Process doesn't exist or we don't have permission
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        assert!(lock2_result.is_err(), "Second instance should fail to acquire lock");
        
        // Verify error message indicates another instance is running
        let error = lock2_result.unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        assert!(error.to_string().contains("already running"));
        
        // Drop first lock
        drop(lock1);
        
        // Verify PID file was removed
        assert!(!test_lock_path.exists(), "PID file should be removed after drop");
        
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
        assert_eq!(successful_count, 1, "Exactly one thread should acquire the lock");
        
        // All others should fail with AlreadyExists error
        let failed_count = results.iter().filter(|r| r.is_err()).count();
        assert_eq!(failed_count, 4, "Four threads should fail to acquire the lock");
        
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
}