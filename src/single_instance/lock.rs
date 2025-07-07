//! Provides the `SingleInstanceLock` struct for acquiring and managing a single instance lock.

use crate::single_instance::path::get_system_wide_lock_path;
use crate::single_instance::process::is_process_running;
use nix::fcntl::{Flock, FlockArg};
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;

/// Represents a single instance lock using a PID file.
#[derive(Debug)]
pub struct SingleInstanceLock {
    /// The locked file handle.
    _flock: Flock<File>,
    /// Path to the PID file.
    pid_file_path: std::path::PathBuf,
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
    pub fn acquire() -> io::Result<Self> {
        let pid_file_path = get_system_wide_lock_path()?;
        Self::acquire_with_path(pid_file_path)
    }

    /// Acquires a lock with a custom file path (for testing).
    #[cfg(test)]
    pub fn acquire_with_test_path<P: AsRef<std::path::Path>>(pid_file_path: P) -> io::Result<Self> {
        Self::acquire_with_path(pid_file_path.as_ref().to_path_buf())
    }

    /// Internal function that handles the actual lock acquisition logic.
    fn acquire_with_path(pid_file_path: std::path::PathBuf) -> io::Result<Self> {
        // Open or create the PID file
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
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
                        format!(
                            "Another instance is already running with PID {}",
                            existing_pid
                        ),
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

    /// Returns the path to the PID file.
    pub fn pid_file_path(&self) -> &Path {
        &self.pid_file_path
    }
}

impl Drop for SingleInstanceLock {
    /// Automatically releases the lock and cleans up the PID file when dropped.
    fn drop(&mut self) {
        // The file lock is automatically released when the file is closed
        // Remove the PID file
        if let Err(e) = std::fs::remove_file(&self.pid_file_path) {
            eprintln!("Warning: Failed to remove PID file: {}", e);
        }
    }
}
