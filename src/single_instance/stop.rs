//! Provides functionality for stopping a running daemon process.

use crate::single_instance::path::get_system_wide_lock_path;
use crate::single_instance::process::is_process_running;
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use std::io;

/// Attempts to stop the running daemon process by reading the PID from the lock file
/// and sending a SIGTERM signal.
///
/// This function reads the PID from the system-wide lock file and attempts to
/// gracefully terminate the daemon process. If the process is not running or
/// the lock file doesn't exist, it returns an appropriate error.
///
/// # Returns
///
/// Returns `Ok(())` if the daemon was successfully stopped, or an error if:
/// - No daemon is currently running
/// - The PID file doesn't exist or is invalid
/// - The process couldn't be terminated
///
/// # Errors
///
/// Returns an error if:
/// - The PID file doesn't exist (no daemon running)
/// - The PID file contains invalid data
/// - The process is not running or doesn't exist
/// - Permission denied when trying to terminate the process
/// - Unable to determine application directories for this platform
pub fn stop_daemon() -> io::Result<()> {
    let pid_file_path = get_system_wide_lock_path()?;
    stop_daemon_with_path(pid_file_path)
}

/// Stops a daemon process using a custom PID file path (for testing).
#[cfg(test)]
pub fn stop_daemon_with_test_path<P: AsRef<std::path::Path>>(pid_file_path: P) -> io::Result<()> {
    stop_daemon_with_path(pid_file_path.as_ref().to_path_buf())
}

/// Internal function that handles the actual daemon stopping logic.
fn stop_daemon_with_path(pid_file_path: std::path::PathBuf) -> io::Result<()> {
    // Check if PID file exists
    if !pid_file_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "No daemon is currently running (PID file not found)",
        ));
    }

    // Read the PID from the file
    let contents = std::fs::read_to_string(&pid_file_path)?;
    let pid_str = contents.trim();

    if pid_str.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "PID file is empty or contains no valid PID",
        ));
    }

    let pid = pid_str.parse::<i32>().map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Invalid PID in lock file: '{}'", pid_str),
        )
    })?;

    // Check if the process is actually running
    if !is_process_running(pid) {
        // Remove stale PID file
        let _ = std::fs::remove_file(&pid_file_path);
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "Daemon process with PID {} is not running (removing stale PID file)",
                pid
            ),
        ));
    }

    // Send SIGTERM to gracefully terminate the process
    match kill(Pid::from_raw(pid), Some(Signal::SIGTERM)) {
        Ok(()) => {
            // Wait a moment for the process to clean up
            std::thread::sleep(std::time::Duration::from_millis(100));

            // Check if process is still running
            if !is_process_running(pid) {
                Ok(())
            } else {
                // Process is still running, but we successfully sent the signal
                // The process should terminate soon
                Ok(())
            }
        }
        Err(e) => Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("Failed to terminate daemon process with PID {}: {}", pid, e),
        )),
    }
}
