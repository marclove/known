//! AGENTS.md file creation and management functionality.

use std::fs;
use std::io;
use std::path::Path;

use crate::constants::{AGENTS_CONTENTS, AGENTS_FILENAME, CLAUDE_FILENAME, GEMINI_FILENAME};
use crate::symlinks::ensure_rules_directory_exists;

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
/// - Creates a .rules directory if it doesn't exist
///
/// # Errors
///
/// Returns an error if:
/// - The current directory cannot be determined
/// - Directory reading fails
/// - File operations (rename/write) fail
/// - Directory creation fails
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
/// 4. If `gemini.md` exists, renames it to `AGENTS.md`
/// 5. If both `claude.md` and `gemini.md` exist, creates empty `AGENTS.md` and prints instructions
/// 6. Otherwise, creates an empty `AGENTS.md` file
/// 7. Creates a `.rules` directory if it doesn't exist
///
/// # Errors
///
/// Returns an error if:
/// - The directory cannot be read
/// - File rename operation fails
/// - File creation fails
/// - Directory creation fails
///
pub fn create_agents_file_in_dir<P: AsRef<Path>>(dir: P) -> io::Result<()> {
    let dir = dir.as_ref();

    let mut agents_exists = false;
    let mut claude_path = None;
    let mut gemini_path = None;

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let file_name = entry.file_name();
        let file_name_str = file_name.to_string_lossy().to_lowercase();

        if file_name_str == AGENTS_FILENAME.to_lowercase() {
            agents_exists = true;
            break;
        } else if file_name_str == CLAUDE_FILENAME.to_lowercase() {
            claude_path = Some(entry.path());
        } else if file_name_str == GEMINI_FILENAME.to_lowercase() {
            gemini_path = Some(entry.path());
        }
    }

    if agents_exists {
        return Ok(());
    }

    let agents_path = dir.join(AGENTS_FILENAME);

    match (claude_path, gemini_path) {
        (Some(_), Some(_)) => {
            // Both CLAUDE.md and GEMINI.md exist
            fs::write(agents_path, "")?;
            println!("Found both CLAUDE.md and GEMINI.md files in the directory.");
            println!("An empty AGENTS.md file has been created.");
            println!("Please manually copy the content from CLAUDE.md and GEMINI.md into AGENTS.md as needed.");
        }
        (Some(claude_file), None) => {
            // Only CLAUDE.md exists
            fs::rename(claude_file, agents_path)?;
        }
        (None, Some(gemini_file)) => {
            // Only GEMINI.md exists
            fs::rename(gemini_file, agents_path)?;
        }
        (None, None) => {
            // Neither exists
            fs::write(agents_path, AGENTS_CONTENTS)?;
        }
    }

    // Create .rules directory if it doesn't exist
    ensure_rules_directory_exists(dir)?;

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
        assert_eq!(content, AGENTS_CONTENTS);

        // Test that .rules directory is created
        let rules_path = dir.path().join(".rules");
        assert!(rules_path.exists());
        assert!(rules_path.is_dir());

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

    #[test]
    fn test_rename_gemini_to_agents() {
        let dir = tempdir().unwrap();

        let gemini_path = dir.path().join(GEMINI_FILENAME);
        fs::write(&gemini_path, "# Gemini content").unwrap();

        let result = create_agents_file_in_dir(dir.path());
        assert!(result.is_ok());

        let agents_path = dir.path().join(AGENTS_FILENAME);

        assert!(agents_path.exists());
        assert!(!gemini_path.exists());

        let content = fs::read_to_string(&agents_path).unwrap();
        assert_eq!(content, "# Gemini content");
    }

    #[test]
    fn test_both_claude_and_gemini_exist() {
        let dir = tempdir().unwrap();

        let claude_path = dir.path().join(CLAUDE_FILENAME);
        let gemini_path = dir.path().join(GEMINI_FILENAME);
        fs::write(&claude_path, "# Claude content").unwrap();
        fs::write(&gemini_path, "# Gemini content").unwrap();

        let result = create_agents_file_in_dir(dir.path());
        assert!(result.is_ok());

        let agents_path = dir.path().join(AGENTS_FILENAME);

        assert!(agents_path.exists());
        assert!(claude_path.exists());
        assert!(gemini_path.exists());

        let content = fs::read_to_string(&agents_path).unwrap();
        assert_eq!(content, "");
    }
}
