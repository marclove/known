//! Provides functionality for checking if a process is running.

use nix::sys::signal::kill;
use nix::unistd::Pid;

/// Checks if a process with the given PID is currently running.
///
/// # Arguments
///
/// * `pid` - The process ID to check.
///
/// # Returns
///
/// Returns `true` if the process exists, `false` otherwise.
pub(crate) fn is_process_running(pid: i32) -> bool {
    match kill(Pid::from_raw(pid), None) {
        Ok(()) => true,  // Process exists
        Err(_) => false, // Process doesn't exist or we don't have permission
    }
}
