//! Single instance enforcement functionality for daemons.
//!
//! This module provides functionality to ensure only one instance of a daemon
//! process can run at a time using PID files and file locking.

use nix::fcntl::{Flock, FlockArg};
use nix::sys::signal::kill;
use nix::unistd::Pid;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;

/// The name of the PID file used for single instance enforcement
const PID_FILE_NAME: &str = ".known_daemon.pid";

/// Represents a single instance lock using a PID file
#[derive(Debug)]
pub struct SingleInstanceLock {
    /// The locked file handle
    _flock: Flock<File>,
    /// Path to the PID file
    pid_file_path: std::path::PathBuf,
}

impl SingleInstanceLock {
    /// Attempts to acquire a single instance lock for the daemon process.
    ///
    /// This function creates or opens a PID file in the specified directory
    /// and attempts to acquire an exclusive lock on it. If successful, it
    /// writes the current process ID to the file.
    ///
    /// # Arguments
    ///
    /// * `dir` - The directory where the PID file should be created
    ///
    /// # Returns
    ///
    /// Returns `Ok(SingleInstanceLock)` if the lock was successfully acquired,
    /// or an error if another instance is already running or if file operations fail.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Another instance of the daemon is already running
    /// - File operations fail (permissions, disk space, etc.)
    /// - The PID file contains an invalid process ID
    ///
    pub fn acquire<P: AsRef<Path>>(dir: P) -> io::Result<Self> {
        let pid_file_path = dir.as_ref().join(PID_FILE_NAME);
        
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
        let dir = tempdir().unwrap();
        
        // First instance should successfully acquire lock
        let lock1 = SingleInstanceLock::acquire(dir.path()).unwrap();
        
        // Verify PID file was created
        let pid_file_path = dir.path().join(PID_FILE_NAME);
        assert!(pid_file_path.exists(), "PID file should be created");
        
        // Verify PID file contains current process ID
        let contents = std::fs::read_to_string(&pid_file_path).unwrap();
        let stored_pid: u32 = contents.trim().parse().unwrap();
        assert_eq!(stored_pid, std::process::id());
        
        // Second instance should fail to acquire lock
        let lock2_result = SingleInstanceLock::acquire(dir.path());
        assert!(lock2_result.is_err(), "Second instance should fail to acquire lock");
        
        // Verify error message indicates another instance is running
        let error = lock2_result.unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        assert!(error.to_string().contains("already running"));
        
        // Drop first lock
        drop(lock1);
        
        // Verify PID file was removed
        assert!(!pid_file_path.exists(), "PID file should be removed after drop");
        
        // Third instance should now succeed
        let lock3 = SingleInstanceLock::acquire(dir.path()).unwrap();
        assert!(pid_file_path.exists(), "PID file should be created again");
        
        drop(lock3);
    }
    
    #[test]
    fn test_stale_pid_file_handling() {
        let dir = tempdir().unwrap();
        let pid_file_path = dir.path().join(PID_FILE_NAME);
        
        // Create a stale PID file with a non-existent PID
        let fake_pid = 999999; // Very unlikely to be a real PID
        std::fs::write(&pid_file_path, format!("{}\n", fake_pid)).unwrap();
        
        // Should still be able to acquire lock despite stale PID file
        let lock = SingleInstanceLock::acquire(dir.path()).unwrap();
        
        // Verify the PID file now contains the current process ID
        let contents = std::fs::read_to_string(&pid_file_path).unwrap();
        let stored_pid: u32 = contents.trim().parse().unwrap();
        assert_eq!(stored_pid, std::process::id());
        
        drop(lock);
    }
    
    #[test]
    fn test_concurrent_lock_acquisition() {
        let dir = tempdir().unwrap();
        let dir_path = dir.path().to_path_buf();
        
        // Spawn multiple threads trying to acquire lock simultaneously
        let handles: Vec<_> = (0..5)
            .map(|i| {
                let dir_clone = dir_path.clone();
                thread::spawn(move || {
                    thread::sleep(Duration::from_millis(i * 10)); // Slight delay variation
                    SingleInstanceLock::acquire(dir_clone)
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
}