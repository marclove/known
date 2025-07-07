//! Provides functionality for determining the system-wide lock file path.

use directories::ProjectDirs;
use std::io;

/// The name of the PID file used for single instance enforcement.
pub(crate) const PID_FILE_NAME: &str = "known_daemon.pid";

/// Gets the system-wide lock file path using the `directories` crate.
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
pub(crate) fn get_system_wide_lock_path() -> io::Result<std::path::PathBuf> {
    let project_dirs = ProjectDirs::from("", "", "known").ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::Other,
            "Unable to determine application directories for this platform",
        )
    })?;

    let data_dir = project_dirs.data_dir();

    // Create the data directory if it doesn't exist
    std::fs::create_dir_all(data_dir)?;

    Ok(data_dir.join(PID_FILE_NAME))
}
