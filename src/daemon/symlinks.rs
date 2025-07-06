//! Symlink management operations for the daemon.

use std::fs;
use std::io;
use std::path::Path;

use crate::constants::{CURSOR_RULES_DIR, WINDSURF_RULES_DIR};
use crate::symlinks::create_symlink_to_file;

/// Creates symlinks for all files in a rules directory to the target directories.
///
/// This function synchronizes the contents of a .rules directory with the target
/// directories (.cursor/rules and .windsurf/rules) by creating symlinks for all
/// files found in the source directory.
///
/// # Arguments
///
/// * `rules_path` - Path to the .rules directory containing the source files
/// * `cursor_rules_path` - Path to the .cursor/rules directory for symlinks
/// * `windsurf_rules_path` - Path to the .windsurf/rules directory for symlinks
///
/// # Errors
///
/// Returns an error if directory creation fails or symlink creation fails for any file.
pub fn sync_rules_directory(
    rules_path: &Path,
    cursor_rules_path: &Path,
    windsurf_rules_path: &Path,
) -> io::Result<()> {
    // Create target directories if they don't exist
    fs::create_dir_all(cursor_rules_path)?;
    fs::create_dir_all(windsurf_rules_path)?;

    // Create symlinks for all existing files in .rules
    for entry in fs::read_dir(rules_path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            let file_name = path.file_name().unwrap();
            let cursor_target = cursor_rules_path.join(file_name);
            let windsurf_target = windsurf_rules_path.join(file_name);

            create_symlink_to_file(&path, &cursor_target)?;
            create_symlink_to_file(&path, &windsurf_target)?;
        }
    }

    Ok(())
}

/// Removes all symlinks from the target directories for a given project directory.
///
/// This function removes all symlinks in both .cursor/rules and .windsurf/rules
/// directories for the specified project directory. It only removes the symlinks,
/// not the original files in .rules.
///
/// # Arguments
///
/// * `dir` - Path to the project directory containing the .rules directory
///
/// # Errors
///
/// Returns an error if directory operations or file removal fails
pub fn remove_symlinks_from_directory(dir: &Path) -> io::Result<()> {
    let cursor_rules_path = dir.join(CURSOR_RULES_DIR);
    let windsurf_rules_path = dir.join(WINDSURF_RULES_DIR);

    // Remove all files from .cursor/rules if it exists
    if cursor_rules_path.exists() {
        for entry in fs::read_dir(&cursor_rules_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                fs::remove_file(&path)?;
            }
        }
    }

    // Remove all files from .windsurf/rules if it exists
    if windsurf_rules_path.exists() {
        for entry in fs::read_dir(&windsurf_rules_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                fs::remove_file(&path)?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::RULES_DIR;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_sync_rules_directory() {
        let dir = tempdir().unwrap();
        let rules_path = dir.path().join(RULES_DIR);
        let cursor_rules_path = dir.path().join(CURSOR_RULES_DIR);
        let windsurf_rules_path = dir.path().join(WINDSURF_RULES_DIR);

        // Create .rules directory and test files
        fs::create_dir(&rules_path).unwrap();
        fs::write(rules_path.join("test1.md"), "content1").unwrap();
        fs::write(rules_path.join("test2.md"), "content2").unwrap();

        // Call sync_rules_directory
        sync_rules_directory(&rules_path, &cursor_rules_path, &windsurf_rules_path).unwrap();

        // Verify target directories were created
        assert!(cursor_rules_path.exists());
        assert!(windsurf_rules_path.exists());

        // Verify symlinks were created
        assert!(cursor_rules_path.join("test1.md").exists());
        assert!(cursor_rules_path.join("test2.md").exists());
        assert!(windsurf_rules_path.join("test1.md").exists());
        assert!(windsurf_rules_path.join("test2.md").exists());
    }

    #[test]
    fn test_remove_symlinks_from_directory_comprehensive() {
        let dir = tempdir().unwrap();
        let rules_path = dir.path().join(RULES_DIR);
        let cursor_rules_path = dir.path().join(CURSOR_RULES_DIR);
        let windsurf_rules_path = dir.path().join(WINDSURF_RULES_DIR);

        // Create .rules directory and test files
        fs::create_dir(&rules_path).unwrap();
        fs::write(rules_path.join("test1.md"), "content1").unwrap();
        fs::write(rules_path.join("test2.md"), "content2").unwrap();

        // Create target directories and symlinks
        fs::create_dir_all(&cursor_rules_path).unwrap();
        fs::create_dir_all(&windsurf_rules_path).unwrap();
        create_symlink_to_file(&rules_path.join("test1.md"), &cursor_rules_path.join("test1.md")).unwrap();
        create_symlink_to_file(&rules_path.join("test2.md"), &cursor_rules_path.join("test2.md")).unwrap();
        create_symlink_to_file(&rules_path.join("test1.md"), &windsurf_rules_path.join("test1.md")).unwrap();
        create_symlink_to_file(&rules_path.join("test2.md"), &windsurf_rules_path.join("test2.md")).unwrap();

        // Verify symlinks exist
        assert!(cursor_rules_path.join("test1.md").exists());
        assert!(cursor_rules_path.join("test2.md").exists());
        assert!(windsurf_rules_path.join("test1.md").exists());
        assert!(windsurf_rules_path.join("test2.md").exists());

        // Call remove_symlinks_from_directory
        remove_symlinks_from_directory(dir.path()).unwrap();

        // Verify symlinks were removed
        assert!(!cursor_rules_path.join("test1.md").exists());
        assert!(!cursor_rules_path.join("test2.md").exists());
        assert!(!windsurf_rules_path.join("test1.md").exists());
        assert!(!windsurf_rules_path.join("test2.md").exists());

        // Verify original files still exist
        assert!(rules_path.join("test1.md").exists());
        assert!(rules_path.join("test2.md").exists());
    }

    #[test]
    fn test_remove_symlinks_from_directory_nonexistent_directories() {
        let dir = tempdir().unwrap();
        
        // Call remove_symlinks_from_directory on a directory without target directories
        let result = remove_symlinks_from_directory(dir.path());
        
        // Should not error when target directories don't exist
        assert!(result.is_ok());
    }

    #[test]
    fn test_sync_rules_directory_with_subdirectories() {
        let dir = tempdir().unwrap();
        let rules_path = dir.path().join(RULES_DIR);
        let cursor_rules_path = dir.path().join(CURSOR_RULES_DIR);
        let windsurf_rules_path = dir.path().join(WINDSURF_RULES_DIR);

        // Create .rules directory with files and subdirectories
        fs::create_dir(&rules_path).unwrap();
        fs::write(rules_path.join("file1.md"), "content1").unwrap();
        fs::create_dir(rules_path.join("subdir")).unwrap();
        fs::write(rules_path.join("subdir").join("file2.md"), "content2").unwrap();

        // Call sync_rules_directory
        sync_rules_directory(&rules_path, &cursor_rules_path, &windsurf_rules_path).unwrap();

        // Verify only files (not subdirectories) were symlinked
        assert!(cursor_rules_path.join("file1.md").exists());
        assert!(windsurf_rules_path.join("file1.md").exists());
        assert!(!cursor_rules_path.join("subdir").exists());
        assert!(!windsurf_rules_path.join("subdir").exists());
    }

    #[test]
    fn test_remove_symlinks_from_directory_with_subdirectories() {
        let dir = tempdir().unwrap();
        let cursor_rules_path = dir.path().join(CURSOR_RULES_DIR);
        let windsurf_rules_path = dir.path().join(WINDSURF_RULES_DIR);

        // Create target directories with files and subdirectories
        fs::create_dir_all(&cursor_rules_path).unwrap();
        fs::create_dir_all(&windsurf_rules_path).unwrap();
        fs::write(cursor_rules_path.join("file1.md"), "content1").unwrap();
        fs::write(windsurf_rules_path.join("file1.md"), "content1").unwrap();
        fs::create_dir(cursor_rules_path.join("subdir")).unwrap();
        fs::create_dir(windsurf_rules_path.join("subdir")).unwrap();
        fs::write(cursor_rules_path.join("subdir").join("file2.md"), "content2").unwrap();
        fs::write(windsurf_rules_path.join("subdir").join("file2.md"), "content2").unwrap();

        // Call remove_symlinks_from_directory
        remove_symlinks_from_directory(dir.path()).unwrap();

        // Verify only files (not subdirectories) were removed
        assert!(!cursor_rules_path.join("file1.md").exists());
        assert!(!windsurf_rules_path.join("file1.md").exists());
        assert!(cursor_rules_path.join("subdir").exists());
        assert!(windsurf_rules_path.join("subdir").exists());
        assert!(cursor_rules_path.join("subdir").join("file2.md").exists());
        assert!(windsurf_rules_path.join("subdir").join("file2.md").exists());
    }
}