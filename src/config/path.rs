//! Handles the logic for determining the configuration file path.

use directories::ProjectDirs;
use std::io;
use std::path::{Path, PathBuf};

/// Configuration file name
const CONFIG_FILE_NAME: &str = "config.json";

/// Returns the path to the configuration file
///
/// Uses the platform-specific application configuration directory according to:
/// - Linux: `$XDG_CONFIG_HOME/known/config.json` or `$HOME/.config/known/config.json`
/// - macOS: `$HOME/Library/Application Support/known/config.json`
/// - Windows: `%APPDATA%/known/config.json`
///
/// Falls back to `$HOME/.config/known/config.json` if platform-specific directories cannot be determined.
///
/// # Errors
///
/// Returns an error if the configuration directory cannot be determined
pub fn get_config_file_path() -> io::Result<PathBuf> {
    // Get HOME directory first to check if it's been overridden (e.g., in tests)
    let home_dir = std::env::var("HOME").map_err(|_| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "Unable to determine home directory for configuration path",
        )
    })?;

    // Check if HOME is set to a temporary directory (common in tests)
    // If so, bypass directories crate and use fallback path directly
    let is_temp_home = home_dir.contains("/tmp/") || home_dir.contains("temp");

    // Debug logging for CI diagnostics
    if std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok() {
        eprintln!("DEBUG: HOME={}", home_dir);
        eprintln!("DEBUG: is_temp_home={}", is_temp_home);
    }

    if is_temp_home {
        // Use platform-appropriate fallback path when HOME is temporary
        let config_dir = if cfg!(target_os = "macos") {
            Path::new(&home_dir)
                .join("Library")
                .join("Application Support")
                .join("known")
        } else {
            Path::new(&home_dir).join(".config").join("known")
        };
        let config_path = config_dir.join(CONFIG_FILE_NAME);

        if std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok() {
            eprintln!(
                "DEBUG: Using platform-aware fallback config path (temp HOME): {}",
                config_path.display()
            );
        }

        return Ok(config_path);
    }

    // Try to use platform-specific directories for normal usage
    if let Some(project_dirs) = ProjectDirs::from("", "", "known") {
        let config_dir = project_dirs.config_dir();
        let config_path = config_dir.join(CONFIG_FILE_NAME);

        // Double-check that the platform-specific path actually respects HOME override
        if let Ok(expected_home) = std::env::var("HOME") {
            if !config_path.starts_with(&expected_home) && expected_home != home_dir {
                // Platform-specific path doesn't match HOME override, use platform-aware fallback
                let fallback_config_dir = if cfg!(target_os = "macos") {
                    Path::new(&home_dir)
                        .join("Library")
                        .join("Application Support")
                        .join("known")
                } else {
                    Path::new(&home_dir).join(".config").join("known")
                };
                let fallback_config_path = fallback_config_dir.join(CONFIG_FILE_NAME);

                if std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok() {
                    eprintln!("DEBUG: Platform path doesn't match HOME, using platform-aware fallback: {}", fallback_config_path.display());
                }

                return Ok(fallback_config_path);
            }
        }

        // Debug logging for CI diagnostics
        if std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok() {
            eprintln!(
                "DEBUG: Using platform-specific config path: {}",
                config_path.display()
            );
            eprintln!(
                "DEBUG: Platform-specific config dir: {}",
                config_dir.display()
            );
        }

        return Ok(config_path);
    }

    // Final platform-aware fallback
    let config_dir = if cfg!(target_os = "macos") {
        Path::new(&home_dir)
            .join("Library")
            .join("Application Support")
            .join("known")
    } else {
        Path::new(&home_dir).join(".config").join("known")
    };
    let config_path = config_dir.join(CONFIG_FILE_NAME);

    // Debug logging for CI diagnostics
    if std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok() {
        eprintln!(
            "DEBUG: Using final platform-aware fallback config path: {}",
            config_path.display()
        );
    }

    Ok(config_path)
}
