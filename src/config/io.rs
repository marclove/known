//! Handles all file input/output operations for the configuration.

use super::path::get_config_file_path;
use super::structure::Config;
use std::fs;
use std::io;
use std::path::Path;

/// Loads configuration from the configuration file
///
/// # Errors
///
/// Returns an error if:
/// - The configuration directory cannot be determined
/// - File reading fails
/// - JSON parsing fails
pub fn load_config() -> io::Result<Config> {
    let config_path = get_config_file_path()?;

    // If config file doesn't exist, return default config
    if !config_path.exists() {
        return Ok(Config::default());
    }

    let config_content = fs::read_to_string(&config_path)?;

    // Trim any trailing whitespace or newlines that might cause parsing issues
    let trimmed_content = config_content.trim();

    // Handle empty file case
    if trimmed_content.is_empty() {
        return Ok(Config::default());
    }

    let config: Config = serde_json::from_str(trimmed_content).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Failed to parse configuration file at {}: {} (content length: {} chars)",
                config_path.display(),
                e,
                trimmed_content.len()
            ),
        )
    })?;

    Ok(config)
}

/// Saves configuration to the configuration file
///
/// # Arguments
///
/// * `config` - The configuration to save
///
/// # Errors
///
/// Returns an error if:
/// - The configuration directory cannot be determined
/// - Directory creation fails
/// - JSON serialization fails
/// - File writing fails
pub fn save_config(config: &Config) -> io::Result<()> {
    let config_path = get_config_file_path()?;

    // Create the configuration directory if it doesn't exist
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut config_content = serde_json::to_string_pretty(config).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to serialize configuration: {}", e),
        )
    })?;

    // Ensure the file ends with a newline
    if !config_content.ends_with('\n') {
        config_content.push('\n');
    }

    fs::write(&config_path, config_content)?;
    Ok(())
}

/// Safely modifies the configuration file using atomic writes to prevent race conditions
///
/// # Arguments
///
/// * `modifier` - Function that modifies the configuration and returns whether changes were made
///
/// # Errors
///
/// Returns an error if file operations fail
fn modify_config_safely<F>(modifier: F) -> io::Result<bool>
where
    F: FnOnce(&mut Config) -> bool,
{
    let config_path = get_config_file_path()?;

    // Debug logging for CI diagnostics
    if std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok() {
        eprintln!(
            "DEBUG: modify_config_safely called with config_path: {}",
            config_path.display()
        );
        if let Some(parent) = config_path.parent() {
            eprintln!("DEBUG: Config parent directory: {}", parent.display());
            eprintln!("DEBUG: Config parent exists: {}", parent.exists());
        }
    }

    // Create the configuration directory if it doesn't exist
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            // Enhanced error for CI debugging
            if std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok() {
                eprintln!(
                    "DEBUG: Failed to create config directory {}: {}",
                    parent.display(),
                    e
                );
            }
            e
        })?;

        if std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok() {
            eprintln!(
                "DEBUG: Successfully created config directory: {}",
                parent.display()
            );
        }
    }

    // Read the current configuration
    let config_content = if config_path.exists() {
        fs::read_to_string(&config_path)?
    } else {
        String::new()
    };

    let mut config = if config_content.trim().is_empty() {
        Config::default()
    } else {
        serde_json::from_str(config_content.trim()).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Failed to parse configuration file at {}: {} (content length: {} chars)",
                    config_path.display(),
                    e,
                    config_content.trim().len()
                ),
            )
        })?
    };

    // Apply the modification
    let changed = modifier(&mut config);

    if changed {
        // Serialize the updated configuration
        let mut new_content = serde_json::to_string_pretty(&config).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to serialize configuration: {}", e),
            )
        })?;

        // Ensure the file ends with a newline
        if !new_content.ends_with('\n') {
            new_content.push('\n');
        }

        // Use atomic write by writing to a temporary file and then renaming
        let temp_path = config_path.with_extension("json.tmp");
        fs::write(&temp_path, new_content)?;
        fs::rename(&temp_path, &config_path)?;
    }

    Ok(changed)
}

/// Adds a directory to the configuration and saves it
///
/// # Arguments
///
/// * `dir_path` - Path to the directory to add
///
/// # Errors
///
/// Returns an error if loading or saving the configuration fails
pub fn add_directory_to_config<P: AsRef<Path>>(dir_path: P) -> io::Result<bool> {
    let dir_path = dir_path.as_ref().to_path_buf();
    modify_config_safely(|config| config.add_directory(&dir_path))
}

/// Removes a directory from the configuration and saves it
///
/// # Arguments
///
/// * `dir_path` - Path to the directory to remove
///
/// # Errors
///
/// Returns an error if loading or saving the configuration fails
pub fn remove_directory_from_config<P: AsRef<Path>>(dir_path: P) -> io::Result<bool> {
    let dir_path = dir_path.as_ref().to_path_buf();
    modify_config_safely(|config| config.remove_directory(&dir_path))
}

/// Loads configuration from a specific file path (for testing)
///
/// # Arguments
///
/// * `config_path` - Path to the configuration file
///
/// # Errors
///
/// Returns an error if file reading or JSON parsing fails
pub fn load_config_from_file<P: AsRef<Path>>(config_path: P) -> io::Result<Config> {
    let config_path = config_path.as_ref();

    // If config file doesn't exist, return default config
    if !config_path.exists() {
        return Ok(Config::default());
    }

    let config_content = fs::read_to_string(config_path)?;

    // Trim any trailing whitespace or newlines that might cause parsing issues
    let trimmed_content = config_content.trim();

    // Handle empty file case
    if trimmed_content.is_empty() {
        return Ok(Config::default());
    }

    let config: Config = serde_json::from_str(trimmed_content).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Failed to parse configuration file at {}: {} (content length: {} chars)",
                config_path.display(),
                e,
                trimmed_content.len()
            ),
        )
    })?;

    Ok(config)
}

/// Saves configuration to a specific file path (for testing)
///
/// # Arguments
///
/// * `config` - The configuration to save
/// * `config_path` - Path to the configuration file
///
/// # Errors
///
/// Returns an error if directory creation, JSON serialization, or file writing fails
pub fn save_config_to_file<P: AsRef<Path>>(config: &Config, config_path: P) -> io::Result<()> {
    let config_path = config_path.as_ref();

    // Create the configuration directory if it doesn't exist
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut config_content = serde_json::to_string_pretty(config).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to serialize configuration: {}", e),
        )
    })?;

    // Ensure the file ends with a newline
    if !config_content.ends_with('\n') {
        config_content.push('\n');
    }

    fs::write(config_path, config_content)?;
    Ok(())
}

/// Safely modifies a specific configuration file using atomic writes (for testing)
///
/// # Arguments
///
/// * `config_path` - Path to the configuration file
/// * `modifier` - Function that modifies the configuration and returns whether changes were made
///
/// # Errors
///
/// Returns an error if file operations fail
fn modify_config_file_safely<F, P: AsRef<Path>>(config_path: P, modifier: F) -> io::Result<bool>
where
    F: FnOnce(&mut Config) -> bool,
{
    let config_path = config_path.as_ref();

    // Create the configuration directory if it doesn't exist
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Read the current configuration
    let config_content = if config_path.exists() {
        fs::read_to_string(config_path)?
    } else {
        String::new()
    };

    let mut config = if config_content.trim().is_empty() {
        Config::default()
    } else {
        serde_json::from_str(config_content.trim()).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Failed to parse configuration file at {}: {} (content length: {} chars)",
                    config_path.display(),
                    e,
                    config_content.trim().len()
                ),
            )
        })?
    };

    // Apply the modification
    let changed = modifier(&mut config);

    if changed {
        // Serialize the updated configuration
        let mut new_content = serde_json::to_string_pretty(&config).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to serialize configuration: {}", e),
            )
        })?;

        // Ensure the file ends with a newline
        if !new_content.ends_with('\n') {
            new_content.push('\n');
        }

        // Use atomic write by writing to a temporary file and then renaming
        let temp_path = config_path.with_extension("json.tmp");
        fs::write(&temp_path, new_content)?;
        fs::rename(&temp_path, config_path)?;
    }

    Ok(changed)
}

/// Adds a directory to a specific configuration file (for testing)
///
/// # Arguments
///
/// * `dir_path` - Path to the directory to add
/// * `config_path` - Path to the configuration file
///
/// # Errors
///
/// Returns an error if loading or saving the configuration fails
pub fn add_directory_to_config_file<P: AsRef<Path>, C: AsRef<Path>>(
    dir_path: P,
    config_path: C,
) -> io::Result<bool> {
    let dir_path = dir_path.as_ref().to_path_buf();
    modify_config_file_safely(config_path, |config| config.add_directory(&dir_path))
}

/// Removes a directory from a specific configuration file (for testing)
///
/// # Arguments
///
/// * `dir_path` - Path to the directory to remove
/// * `config_path` - Path to the configuration file
///
/// # Errors
///
/// Returns an error if loading or saving the configuration fails
pub fn remove_directory_from_config_file<P: AsRef<Path>, C: AsRef<Path>>(
    dir_path: P,
    config_path: C,
) -> io::Result<bool> {
    let dir_path = dir_path.as_ref().to_path_buf();
    modify_config_file_safely(config_path, |config| config.remove_directory(&dir_path))
}
