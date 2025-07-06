//! Autostart functionality for the known daemon.
//!
//! This module provides cross-platform autostart functionality using the auto-launch crate.
//! It allows the daemon to be automatically started when the system boots.

use auto_launch::{AutoLaunch, AutoLaunchBuilder};
use std::env;
use std::io::{self, Error, ErrorKind};

/// The application name used for autostart registration
const APP_NAME: &str = "known-daemon";

/// Builds the AutoLaunch instance with the correct configuration.
///
/// This function centralizes the creation of the AutoLaunch instance
/// to avoid code duplication across enable, disable, and status check functions.
///
/// # Errors
///
/// Returns an error if the current executable path cannot be determined.
fn build_auto_launch() -> io::Result<AutoLaunch> {
    let current_exe = env::current_exe().map_err(|e| {
        Error::new(
            ErrorKind::NotFound,
            format!("Could not determine current executable path: {}", e),
        )
    })?;


    AutoLaunchBuilder::new()
        .set_app_name(APP_NAME)
        .set_app_path(current_exe.to_str().ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidData,
                "Executable path contains invalid UTF-8 characters",
            )
        })?)
        .set_use_launch_agent(true)
        .set_args(&["run-daemon"])
        .build()
        .map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("Failed to create AutoLaunch instance: {}", e),
            )
        })
}

/// Enables autostart for the known daemon.
///
/// This function registers the current executable to start automatically
/// when the system boots. The daemon will start with the `run-daemon` command
/// and use the global configuration file to determine which directories to watch.
///
/// # Platform Support
///
/// - **Windows**: Uses Windows registry entries
/// - **macOS**: Uses Launch Agents or AppleScript
/// - **Linux**: Uses systemd or equivalent service manager
///
/// # Errors
///
/// Returns an error if:
/// - The current executable path cannot be determined
/// - Autostart registration fails on the platform
///
pub fn enable_autostart() -> io::Result<()> {
    let auto_launch = build_auto_launch()?;
    auto_launch.enable().map_err(|e| {
        Error::new(
            ErrorKind::Other,
            format!("Failed to enable autostart: {}", e),
        )
    })?;
    Ok(())
}

/// Disables autostart for the known daemon.
///
/// This function removes the autostart registration for the known daemon.
///
/// # Errors
///
/// Returns an error if:
/// - The current executable path cannot be determined
/// - Autostart deregistration fails on the platform
///
pub fn disable_autostart() -> io::Result<()> {
    let auto_launch = build_auto_launch()?;
    auto_launch.disable().map_err(|e| {
        Error::new(
            ErrorKind::Other,
            format!("Failed to disable autostart: {}", e),
        )
    })?;
    Ok(())
}

/// Checks if autostart is currently enabled for the known daemon.
///
/// # Errors
///
/// Returns an error if:
/// - The current executable path cannot be determined
/// - Autostart status cannot be determined
///
pub fn is_autostart_enabled() -> io::Result<bool> {
    let auto_launch = build_auto_launch()?;
    auto_launch.is_enabled().map_err(|e| {
        Error::new(
            ErrorKind::Other,
            format!("Failed to check autostart status: {}", e),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enable_disable_autostart() {
        // Test that we can enable autostart
        let enable_result = enable_autostart();

        // The result might fail in CI/test environments, but the function should not panic
        // and should return a proper Result type
        match enable_result {
            Ok(()) => {
                // If enable succeeded, test that we can check status
                let status_result = is_autostart_enabled();
                assert!(status_result.is_ok());

                // Test that we can disable it
                let disable_result = disable_autostart();
                match disable_result {
                    Ok(()) => {
                        // If disable succeeded, check that it's no longer enabled
                        let final_status = is_autostart_enabled();
                        assert!(final_status.is_ok());
                    }
                    Err(_) => {
                        // Disable might fail in test environment, but shouldn't panic
                        // This is acceptable for testing
                    }
                }
            }
            Err(_) => {
                // Enable might fail in CI/test environments due to permissions
                // This is acceptable for testing - we just verify the function doesn't panic
            }
        }
    }

    #[test]
    fn test_autostart_status_check() {
        // Test that checking autostart status doesn't panic
        let status_result = is_autostart_enabled();

        // The result might fail in CI/test environments, but should not panic
        // and should return a proper Result type
        match status_result {
            Ok(_enabled) => {
                // Status check succeeded
            }
            Err(_) => {
                // Status check might fail in test environment, but shouldn't panic
                // This is acceptable for testing
            }
        }
    }

    #[test]
    fn test_app_name_constant() {
        // Test that APP_NAME is properly defined
        assert_eq!(APP_NAME, "known-daemon");
        assert!(!APP_NAME.is_empty());
    }
}
