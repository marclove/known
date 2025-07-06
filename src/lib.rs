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
const CLAUDE_FILENAME: &str = "CLAUDE.md";

/// The filename for the gemini instruction file (uppercase)
const GEMINI_FILENAME: &str = "GEMINI.md";

/// The directory name for rules files
const RULES_DIR: &str = ".rules";

/// The directory name for cursor rules files
const CURSOR_RULES_DIR: &str = ".cursor/rules";

/// The directory name for windsurf rules files
const WINDSURF_RULES_DIR: &str = ".windsurf/rules";

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
/// - Moves files from .cursor/rules and .windsurf/rules to .rules directory
/// - Uses platform-specific symlink functions for cross-platform compatibility
///
/// # Errors
///
/// Returns an error if:
/// - AGENTS.md doesn't exist in the target directory
/// - Symlink creation fails
/// - File moving fails
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

    // Create .rules directory if it doesn't exist
    ensure_rules_directory_exists(dir)?;
    let rules_path = dir.join(RULES_DIR);

    // Move files from .cursor/rules to .rules directory
    let cursor_rules_path = dir.join(CURSOR_RULES_DIR);
    move_files_to_rules_dir(&cursor_rules_path, &rules_path)?;

    // Move files from .windsurf/rules to .rules directory
    let windsurf_rules_path = dir.join(WINDSURF_RULES_DIR);
    move_files_to_rules_dir(&windsurf_rules_path, &rules_path)?;

    let claude_path = dir.join(CLAUDE_FILENAME);
    let gemini_path = dir.join(GEMINI_FILENAME);

    // Remove existing symlinks if they exist
    remove_existing_symlinks(&claude_path, &gemini_path)?;

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

/// Ensures the .rules directory exists in the specified directory.
///
/// # Arguments
///
/// * `dir` - The parent directory where .rules should be created
///
/// # Errors
///
/// Returns an error if directory creation fails
///
fn ensure_rules_directory_exists<P: AsRef<Path>>(dir: P) -> io::Result<()> {
    let rules_path = dir.as_ref().join(RULES_DIR);
    if !rules_path.exists() {
        fs::create_dir(rules_path)?;
    }
    Ok(())
}

/// Removes existing symlink files if they exist.
///
/// # Arguments
///
/// * `claude_path` - Path to the CLAUDE.md symlink
/// * `gemini_path` - Path to the GEMINI.md symlink
///
/// # Errors
///
/// Returns an error if file removal fails
///
fn remove_existing_symlinks<P: AsRef<Path>>(claude_path: P, gemini_path: P) -> io::Result<()> {
    let claude_path = claude_path.as_ref();
    let gemini_path = gemini_path.as_ref();

    if claude_path.exists() {
        fs::remove_file(claude_path)?;
    }
    if gemini_path.exists() {
        fs::remove_file(gemini_path)?;
    }
    Ok(())
}

/// Moves files from a source directory to the target .rules directory.
///
/// This function scans the source directory for files and attempts to move them
/// to the target directory. If a file with the same name already exists in the
/// target directory, it prints a warning and skips the file.
///
/// # Arguments
///
/// * `source_dir` - The source directory path to move files from
/// * `target_dir` - The target directory path to move files to
///
/// # Errors
///
/// Returns an error if:
/// - Directory reading fails
/// - File moving fails for reasons other than the target file already existing
///
fn move_files_to_rules_dir<P: AsRef<Path>>(source_dir: P, target_dir: P) -> io::Result<()> {
    let source_dir = source_dir.as_ref();
    let target_dir = target_dir.as_ref();

    // Check if source directory exists
    if !source_dir.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(source_dir)? {
        let entry = entry?;
        let file_name = entry.file_name();
        let source_path = entry.path();
        let target_path = target_dir.join(&file_name);

        // Skip if target file already exists
        if target_path.exists() {
            println!(
                "Warning: File '{}' already exists in .rules directory. Skipping.",
                file_name.to_string_lossy()
            );
            continue;
        }

        // Move the file
        fs::rename(&source_path, &target_path)?;
    }

    Ok(())
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
        let rules_path = dir.path().join(RULES_DIR);
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

    #[test]
    fn test_rules_directory_already_exists() {
        let dir = tempdir().unwrap();

        // Create .rules directory beforehand
        let rules_path = dir.path().join(RULES_DIR);
        fs::create_dir(&rules_path).unwrap();

        // Create agents file - should not fail even if .rules exists
        let result = create_agents_file_in_dir(dir.path());
        assert!(result.is_ok());

        // Verify .rules directory still exists
        assert!(rules_path.exists());
        assert!(rules_path.is_dir());

        // Verify agents file was created
        let agents_path = dir.path().join(AGENTS_FILENAME);
        assert!(agents_path.exists());
    }

    #[test]
    fn test_move_cursor_rules_to_rules_directory() {
        let dir = tempdir().unwrap();

        // Create AGENTS.md file first
        let agents_path = dir.path().join(AGENTS_FILENAME);
        fs::write(&agents_path, "# Agents content").unwrap();

        // Create .cursor/rules directory with test files
        let cursor_rules_path = dir.path().join(".cursor/rules");
        fs::create_dir_all(&cursor_rules_path).unwrap();

        let test_file1 = cursor_rules_path.join("rule1.txt");
        let test_file2 = cursor_rules_path.join("rule2.md");
        fs::write(&test_file1, "Rule 1 content").unwrap();
        fs::write(&test_file2, "Rule 2 content").unwrap();

        // Create .rules directory
        let rules_path = dir.path().join(RULES_DIR);
        fs::create_dir(&rules_path).unwrap();

        // Run symlink command
        let result = create_symlinks_in_dir(dir.path());
        assert!(result.is_ok());

        // Verify files were moved to .rules directory
        let moved_file1 = rules_path.join("rule1.txt");
        let moved_file2 = rules_path.join("rule2.md");
        assert!(moved_file1.exists());
        assert!(moved_file2.exists());

        // Verify original files no longer exist
        assert!(!test_file1.exists());
        assert!(!test_file2.exists());

        // Verify content is preserved
        let content1 = fs::read_to_string(&moved_file1).unwrap();
        let content2 = fs::read_to_string(&moved_file2).unwrap();
        assert_eq!(content1, "Rule 1 content");
        assert_eq!(content2, "Rule 2 content");
    }

    #[test]
    fn test_move_windsurf_rules_to_rules_directory() {
        let dir = tempdir().unwrap();

        // Create AGENTS.md file first
        let agents_path = dir.path().join(AGENTS_FILENAME);
        fs::write(&agents_path, "# Agents content").unwrap();

        // Create .windsurf/rules directory with test files
        let windsurf_rules_path = dir.path().join(".windsurf/rules");
        fs::create_dir_all(&windsurf_rules_path).unwrap();

        let test_file1 = windsurf_rules_path.join("config.toml");
        let test_file2 = windsurf_rules_path.join("settings.json");
        fs::write(&test_file1, "config content").unwrap();
        fs::write(&test_file2, "settings content").unwrap();

        // Create .rules directory
        let rules_path = dir.path().join(RULES_DIR);
        fs::create_dir(&rules_path).unwrap();

        // Run symlink command
        let result = create_symlinks_in_dir(dir.path());
        assert!(result.is_ok());

        // Verify files were moved to .rules directory
        let moved_file1 = rules_path.join("config.toml");
        let moved_file2 = rules_path.join("settings.json");
        assert!(moved_file1.exists());
        assert!(moved_file2.exists());

        // Verify original files no longer exist
        assert!(!test_file1.exists());
        assert!(!test_file2.exists());

        // Verify content is preserved
        let content1 = fs::read_to_string(&moved_file1).unwrap();
        let content2 = fs::read_to_string(&moved_file2).unwrap();
        assert_eq!(content1, "config content");
        assert_eq!(content2, "settings content");
    }

    #[test]
    fn test_move_rules_skip_existing_files() {
        let dir = tempdir().unwrap();

        // Create AGENTS.md file first
        let agents_path = dir.path().join(AGENTS_FILENAME);
        fs::write(&agents_path, "# Agents content").unwrap();

        // Create .cursor/rules directory with test files
        let cursor_rules_path = dir.path().join(".cursor/rules");
        fs::create_dir_all(&cursor_rules_path).unwrap();

        let test_file = cursor_rules_path.join("duplicate.txt");
        fs::write(&test_file, "cursor content").unwrap();

        // Create .rules directory with existing file
        let rules_path = dir.path().join(RULES_DIR);
        fs::create_dir(&rules_path).unwrap();
        let existing_file = rules_path.join("duplicate.txt");
        fs::write(&existing_file, "existing content").unwrap();

        // Run symlink command
        let result = create_symlinks_in_dir(dir.path());
        assert!(result.is_ok());

        // Verify original file still exists in cursor/rules
        assert!(test_file.exists());

        // Verify existing file content is preserved
        let content = fs::read_to_string(&existing_file).unwrap();
        assert_eq!(content, "existing content");
    }

    #[test]
    fn test_move_rules_no_source_directories() {
        let dir = tempdir().unwrap();

        // Create AGENTS.md file first
        let agents_path = dir.path().join(AGENTS_FILENAME);
        fs::write(&agents_path, "# Agents content").unwrap();

        // Create .rules directory
        let rules_path = dir.path().join(RULES_DIR);
        fs::create_dir(&rules_path).unwrap();

        // Run symlink command without .cursor or .windsurf directories
        let result = create_symlinks_in_dir(dir.path());
        assert!(result.is_ok());

        // Verify symlinks were still created
        let claude_path = dir.path().join(CLAUDE_FILENAME);
        let gemini_path = dir.path().join(GEMINI_FILENAME);
        assert!(claude_path.exists());
        assert!(gemini_path.exists());
    }
}
