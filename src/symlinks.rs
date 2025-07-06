//! Symlink management and rules directory operations.

use std::fs;
use std::io;
use std::path::Path;

use crate::config::add_directory_to_config;

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
    let agents_path = dir.join("AGENTS.md");

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

    let claude_path = dir.join("CLAUDE.md");
    let gemini_path = dir.join("GEMINI.md");

    // Remove existing symlinks if they exist
    remove_existing_symlinks(&claude_path, &gemini_path)?;

    // Create symlinks using platform-specific functions
    let agents_symlink_target = Path::new("AGENTS.md");
    create_platform_symlink(agents_symlink_target, &claude_path)?;
    create_platform_symlink(agents_symlink_target, &gemini_path)?;

    // Add directory to configuration file for daemon tracking
    if let Err(e) = add_directory_to_config(dir) {
        eprintln!("Warning: Failed to add directory to config: {}", e);
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
pub fn ensure_rules_directory_exists<P: AsRef<Path>>(dir: P) -> io::Result<()> {
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

/// Creates a platform-specific symlink.
///
/// # Arguments
///
/// * `source` - Path to the source file
/// * `target` - Path where the symlink should be created
///
/// # Errors
///
/// Returns an error if symlink creation fails
///
fn create_platform_symlink(source: &Path, target: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(source, target)?;
    }

    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_file(source, target)?;
    }

    Ok(())
}

/// Creates a symlink from target to source file.
///
/// # Arguments
///
/// * `source` - Path to the source file
/// * `target` - Path where the symlink should be created
///
/// # Errors
///
/// Returns an error if symlink creation fails
///
pub fn create_symlink_to_file(source: &Path, target: &Path) -> io::Result<()> {
    // Remove existing symlink if it exists
    if target.exists() {
        fs::remove_file(target)?;
    }

    // Create platform-specific symlink
    create_platform_symlink(source, target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_create_symlinks_success() {
        let dir = tempdir().unwrap();

        // First create an AGENTS.md file
        let agents_path = dir.path().join("AGENTS.md");
        fs::write(&agents_path, "# Agents content").unwrap();

        // Create symlinks
        let result = create_symlinks_in_dir(dir.path());
        assert!(result.is_ok());

        // Check that symlinks were created
        let claude_path = dir.path().join("CLAUDE.md");
        let gemini_path = dir.path().join("GEMINI.md");

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
        let agents_path = dir.path().join("AGENTS.md");
        fs::write(&agents_path, "# Agents content").unwrap();

        // Create existing files that will be overwritten
        let claude_path = dir.path().join("CLAUDE.md");
        let gemini_path = dir.path().join("GEMINI.md");
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

        // Create AGENTS.md file first
        let agents_path = dir.path().join("AGENTS.md");
        fs::write(&agents_path, "# Agents content").unwrap();

        // Create symlinks - should not fail even if .rules exists
        let result = create_symlinks_in_dir(dir.path());
        assert!(result.is_ok());

        // Verify .rules directory still exists
        assert!(rules_path.exists());
        assert!(rules_path.is_dir());
    }

    #[test]
    fn test_move_cursor_rules_to_rules_directory() {
        let dir = tempdir().unwrap();

        // Create AGENTS.md file first
        let agents_path = dir.path().join("AGENTS.md");
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
        let agents_path = dir.path().join("AGENTS.md");
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
        let agents_path = dir.path().join("AGENTS.md");
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
        let agents_path = dir.path().join("AGENTS.md");
        fs::write(&agents_path, "# Agents content").unwrap();

        // Create .rules directory
        let rules_path = dir.path().join(RULES_DIR);
        fs::create_dir(&rules_path).unwrap();

        // Run symlink command without .cursor or .windsurf directories
        let result = create_symlinks_in_dir(dir.path());
        assert!(result.is_ok());

        // Verify symlinks were still created
        let claude_path = dir.path().join("CLAUDE.md");
        let gemini_path = dir.path().join("GEMINI.md");
        assert!(claude_path.exists());
        assert!(gemini_path.exists());
    }
}
