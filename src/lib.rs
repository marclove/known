//! A Rust library for managing project Agentic LLM instruction files.
//!
//! This library provides functionality for creating and managing AGENTS.md files
//! in project directories, with support for renaming existing CLAUDE.md files.

use std::fs;
use std::io;
use std::path::Path;

/// The filename for the agents instruction file (uppercase)
const AGENTS_FILENAME: &str = "AGENTS.md";
const AGENTS_CONTENTS: &str = "# AGENTS\nThis file provides guidance to agentic coding agents like [Claude Code](https://claude.ai/code), [Gemini CLI](https://github.com/google-gemini/gemini-cli), and [Codex CLI](https://github.com/openai/codex) when working with code in this repository.";

/// The filename for the claude instruction file (uppercase)
#[allow(dead_code)]
const CLAUDE_FILENAME: &str = "CLAUDE.md";

/// The filename for the gemini instruction file (uppercase)
#[allow(dead_code)]
const GEMINI_FILENAME: &str = "GEMINI.md";

/// Creates symbolic links from AGENTS.md to CLAUDE.md and GEMINI.md in the current working directory.
///
/// This function creates symlinks that point from CLAUDE.md and GEMINI.md to AGENTS.md,
/// allowing users to maintain compatibility with both naming conventions.
///
/// # Behavior
///
/// - If AGENTS.md doesn't exist, returns an error
/// - Creates symlinks from CLAUDE.md and GEMINI.md to AGENTS.md
/// - Overwrites existing symlinks if they already exist
/// - On Windows, creates file symlinks; on Unix, creates regular symlinks
///
/// # Errors
///
/// Returns an error if:
/// - AGENTS.md doesn't exist in the current directory
/// - Symlink creation fails due to permissions or other OS-level issues
///
pub fn create_symlinks() -> io::Result<()> {
    let current_dir = std::env::current_dir()?;
    create_symlinks_in_dir(&current_dir)
}

/// Creates symbolic links from AGENTS.md to CLAUDE.md and GEMINI.md in the specified directory.
///
/// This is the core function that handles symlink creation logic.
///
/// # Arguments
///
/// * `dir` - The directory path where the symlinks should be created
///
/// # Behavior
///
/// - Verifies that AGENTS.md exists in the target directory
/// - Creates symlinks from CLAUDE.md and GEMINI.md to AGENTS.md
/// - Uses platform-specific symlink functions for cross-platform compatibility
///
/// # Errors
///
/// Returns an error if:
/// - AGENTS.md doesn't exist in the target directory
/// - Symlink creation fails
///
pub fn create_symlinks_in_dir<P: AsRef<Path>>(dir: P) -> io::Result<()> {
    let dir = dir.as_ref();
    let agents_path = dir.join(AGENTS_FILENAME);

    // Check if AGENTS.md exists
    if !agents_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "AGENTS.md file not found. Please run 'known init' first.",
        ));
    }

    let claude_path = dir.join(CLAUDE_FILENAME);
    let gemini_path = dir.join(GEMINI_FILENAME);

    // Remove existing symlinks if they exist
    if claude_path.exists() {
        fs::remove_file(&claude_path)?;
    }
    if gemini_path.exists() {
        fs::remove_file(&gemini_path)?;
    }

    // Create symlinks using platform-specific functions
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(AGENTS_FILENAME, &claude_path)?;
        std::os::unix::fs::symlink(AGENTS_FILENAME, &gemini_path)?;
    }

    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_file(AGENTS_FILENAME, &claude_path)?;
        std::os::windows::fs::symlink_file(AGENTS_FILENAME, &gemini_path)?;
    }

    Ok(())
}

/// Helper function to convert filename to lowercase for case-insensitive comparisons
fn to_lowercase(filename: &str) -> String {
    filename.to_lowercase()
}

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
/// 4. If `gemini.md` exists, renames it to `AGENTS.md`
/// 5. If both `claude.md` and `gemini.md` exist, creates empty `AGENTS.md` and prints instructions
/// 6. Otherwise, creates an empty `AGENTS.md` file
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
    let mut gemini_path = None;

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let file_name = entry.file_name();
        let file_name_str = file_name.to_string_lossy().to_lowercase();

        if file_name_str == to_lowercase(AGENTS_FILENAME) {
            agents_exists = true;
            break;
        } else if file_name_str == to_lowercase(CLAUDE_FILENAME) {
            claude_path = Some(entry.path());
        } else if file_name_str == to_lowercase(GEMINI_FILENAME) {
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

    #[test]
    fn test_create_symlinks_success() {
        let dir = tempdir().unwrap();

        // First create an AGENTS.md file
        let agents_path = dir.path().join(AGENTS_FILENAME);
        fs::write(&agents_path, "# Agents content").unwrap();

        // Create symlinks
        let result = create_symlinks_in_dir(dir.path());
        assert!(result.is_ok());

        // Check that symlinks were created
        let claude_path = dir.path().join(CLAUDE_FILENAME);
        let gemini_path = dir.path().join(GEMINI_FILENAME);

        assert!(claude_path.exists());
        assert!(gemini_path.exists());

        // Verify symlinks point to correct content
        let claude_content = fs::read_to_string(&claude_path).unwrap();
        let gemini_content = fs::read_to_string(&gemini_path).unwrap();
        let agents_content = fs::read_to_string(&agents_path).unwrap();

        assert_eq!(claude_content, agents_content);
        assert_eq!(gemini_content, agents_content);
        assert_eq!(claude_content, "# Agents content");
    }

    #[test]
    fn test_create_symlinks_no_agents_file() {
        let dir = tempdir().unwrap();

        // Try to create symlinks without AGENTS.md
        let result = create_symlinks_in_dir(dir.path());
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
        assert!(error.to_string().contains("AGENTS.md file not found"));
    }

    #[test]
    fn test_create_symlinks_overwrites_existing() {
        let dir = tempdir().unwrap();

        // Create AGENTS.md file
        let agents_path = dir.path().join(AGENTS_FILENAME);
        fs::write(&agents_path, "# Agents content").unwrap();

        // Create existing files that will be overwritten
        let claude_path = dir.path().join(CLAUDE_FILENAME);
        let gemini_path = dir.path().join(GEMINI_FILENAME);
        fs::write(&claude_path, "# Old Claude content").unwrap();
        fs::write(&gemini_path, "# Old Gemini content").unwrap();

        // Create symlinks (should overwrite existing files)
        let result = create_symlinks_in_dir(dir.path());
        assert!(result.is_ok());

        // Verify symlinks point to AGENTS.md content
        let claude_content = fs::read_to_string(&claude_path).unwrap();
        let gemini_content = fs::read_to_string(&gemini_path).unwrap();

        assert_eq!(claude_content, "# Agents content");
        assert_eq!(gemini_content, "# Agents content");
    }
}
