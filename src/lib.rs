//! A Rust library for managing project Agentic LLM instruction files.
//!
//! This library provides functionality for creating and managing AGENTS.md files
//! in project directories, with support for renaming existing CLAUDE.md files.

use std::fs;
use std::io;
use std::path::Path;

/// The filename for the agents instruction file (uppercase)
const AGENTS_FILENAME: &str = "AGENTS.md";

/// The filename for the claude instruction file (uppercase)
#[allow(dead_code)]  
const CLAUDE_FILENAME: &str = "CLAUDE.md";

/// The lowercase version of the agents filename for case-insensitive matching
const AGENTS_FILENAME_LOWER: &str = "agents.md";

/// The lowercase version of the claude filename for case-insensitive matching
const CLAUDE_FILENAME_LOWER: &str = "claude.md";

/// Creates an AGENTS.md file in the current working directory.
///
/// This function is a convenience wrapper around [`create_agents_file_in_dir`] that
/// operates on the current working directory.
///
/// # Behavior
///
/// - If an AGENTS.md file already exists, no action is taken
/// - If a CLAUDE.md file exists, it will be renamed to AGENTS.md
/// - Otherwise, an empty AGENTS.md file is created
///
/// # Errors
///
/// Returns an error if:
/// - The current directory cannot be determined
/// - Directory reading fails
/// - File operations (rename/write) fail
///
pub fn create_agents_file() -> io::Result<()> {
    let current_dir = std::env::current_dir()?;
    create_agents_file_in_dir(&current_dir)
}

/// Creates an AGENTS.md file in the specified directory.
///
/// This is the core function that handles AGENTS.md file creation logic.
/// It performs case-insensitive checks for existing files and handles
/// renaming CLAUDE.md to AGENTS.md if present.
///
/// # Arguments
///
/// * `dir` - The directory path where the AGENTS.md file should be created
///
/// # Behavior
///
/// 1. Scans the directory for existing files (case-insensitive)
/// 2. If `agents.md` exists in any case variation, returns successfully without changes
/// 3. If `claude.md` exists, renames it to `AGENTS.md`
/// 4. Otherwise, creates an empty `AGENTS.md` file
///
/// # Errors
///
/// Returns an error if:
/// - The directory cannot be read
/// - File rename operation fails
/// - File creation fails
///
pub fn create_agents_file_in_dir<P: AsRef<Path>>(dir: P) -> io::Result<()> {
    let dir = dir.as_ref();

    let mut agents_exists = false;
    let mut claude_path = None;

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let file_name = entry.file_name();
        let file_name_str = file_name.to_string_lossy().to_lowercase();

        if file_name_str == AGENTS_FILENAME_LOWER {
            agents_exists = true;
            break;
        } else if file_name_str == CLAUDE_FILENAME_LOWER {
            claude_path = Some(entry.path());
        }
    }

    if agents_exists {
        return Ok(());
    }

    let agents_path = dir.join(AGENTS_FILENAME);

    if let Some(claude_file) = claude_path {
        fs::rename(claude_file, agents_path)?;
    } else {
        fs::write(agents_path, "")?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_create_agents_file() {
        let dir = tempdir().unwrap();

        let result = create_agents_file_in_dir(dir.path());
        assert!(result.is_ok());

        let agents_path = dir.path().join(AGENTS_FILENAME);
        assert!(agents_path.exists());

        let content = fs::read_to_string(&agents_path).unwrap();
        assert_eq!(content, "");

        // Test idempotency
        let result_second = create_agents_file_in_dir(dir.path());
        assert!(result_second.is_ok());
    }

    #[test]
    fn test_rename_claude_to_agents() {
        let dir = tempdir().unwrap();

        let claude_path = dir.path().join(CLAUDE_FILENAME);
        fs::write(&claude_path, "# Test content").unwrap();

        let result = create_agents_file_in_dir(dir.path());
        assert!(result.is_ok());

        let agents_path = dir.path().join(AGENTS_FILENAME);

        assert!(agents_path.exists());
        assert!(!claude_path.exists());

        let content = fs::read_to_string(&agents_path).unwrap();
        assert_eq!(content, "# Test content");
    }

    #[test]
    fn test_agents_already_exists() {
        let dir = tempdir().unwrap();

        let agents_path = dir.path().join(AGENTS_FILENAME);
        fs::write(&agents_path, "existing content").unwrap();

        let result = create_agents_file_in_dir(dir.path());
        assert!(result.is_ok());

        let content = fs::read_to_string(&agents_path).unwrap();
        assert_eq!(content, "existing content");
    }
}
