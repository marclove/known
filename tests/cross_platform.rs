use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_symlink_creation_cross_platform() {
    let temp_dir = tempdir().unwrap();
    let project_dir = temp_dir.path().join("cross_platform_test");
    std::fs::create_dir_all(&project_dir).unwrap();

    // Create AGENTS.md with comprehensive content
    let agents_content = r#"# Cross-Platform Test Project

This is a test project for cross-platform symlink functionality.

## Rules

- Rule 1: Test symlink creation
- Rule 2: Verify cross-platform compatibility
- Rule 3: Check file system handling

## Notes

Testing various file system edge cases:
- Special characters in paths
- Different line endings
- Unicode content: 测试 テスト тест
"#;

    std::fs::write(project_dir.join("AGENTS.md"), agents_content).unwrap();

    // Test symlink command
    let mut cmd = Command::cargo_bin("known").unwrap();
    cmd.current_dir(&project_dir)
        .env("HOME", temp_dir.path())
        .arg("symlink");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Successfully created symlinks"));

    // Verify symlinks exist
    assert!(project_dir.join("CLAUDE.md").exists());
    assert!(project_dir.join("GEMINI.md").exists());

    // Verify symlink content matches original
    let claude_content = std::fs::read_to_string(project_dir.join("CLAUDE.md")).unwrap();
    let gemini_content = std::fs::read_to_string(project_dir.join("GEMINI.md")).unwrap();

    assert_eq!(claude_content, agents_content);
    assert_eq!(gemini_content, agents_content);

    // Test that symlinks are actual symlinks (on Unix) or files (on Windows)
    let claude_metadata = std::fs::symlink_metadata(project_dir.join("CLAUDE.md")).unwrap();
    let gemini_metadata = std::fs::symlink_metadata(project_dir.join("GEMINI.md")).unwrap();

    if cfg!(unix) {
        // On Unix, these should be symlinks
        assert!(claude_metadata.file_type().is_symlink());
        assert!(gemini_metadata.file_type().is_symlink());
    } else {
        // On Windows, these might be files or symlinks depending on permissions
        assert!(claude_metadata.file_type().is_file() || claude_metadata.file_type().is_symlink());
        assert!(gemini_metadata.file_type().is_file() || gemini_metadata.file_type().is_symlink());
    }
}

#[test]
fn test_symlink_with_special_paths() {
    let temp_dir = tempdir().unwrap();

    // Create a project with a name that might cause issues
    let project_name = if cfg!(windows) {
        // Windows has stricter path restrictions
        "test_project_with_spaces"
    } else {
        // Unix systems can handle more special characters
        "test-project_with.special@chars"
    };

    let project_dir = temp_dir.path().join(project_name);
    std::fs::create_dir_all(&project_dir).unwrap();

    // Create AGENTS.md
    std::fs::write(
        project_dir.join("AGENTS.md"),
        "# Special Path Test\n\nTesting special characters in paths.",
    )
    .unwrap();

    // Test symlink creation
    let mut cmd = Command::cargo_bin("known").unwrap();
    cmd.current_dir(&project_dir)
        .env("HOME", temp_dir.path())
        .arg("symlink");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Successfully created symlinks"));

    // Verify symlinks were created successfully
    assert!(project_dir.join("CLAUDE.md").exists());
    assert!(project_dir.join("GEMINI.md").exists());

    // Verify content is correct
    let content = std::fs::read_to_string(project_dir.join("CLAUDE.md")).unwrap();
    assert!(content.contains("Special Path Test"));
}

#[test]
fn test_rules_directory_cross_platform() {
    let temp_dir = tempdir().unwrap();
    let project_dir = temp_dir.path().join("rules_test");
    std::fs::create_dir_all(&project_dir).unwrap();
    std::fs::create_dir_all(project_dir.join(".rules")).unwrap();

    // Create AGENTS.md
    std::fs::write(project_dir.join("AGENTS.md"), "# Rules Directory Test").unwrap();

    // Create test files in .rules directory with different line endings
    let rule_content_unix = "# Unix Rule\nThis rule uses Unix line endings.\n";
    let rule_content_windows = "# Windows Rule\r\nThis rule uses Windows line endings.\r\n";

    std::fs::write(
        project_dir.join(".rules").join("unix_rule.md"),
        rule_content_unix,
    )
    .unwrap();
    std::fs::write(
        project_dir.join(".rules").join("windows_rule.md"),
        rule_content_windows,
    )
    .unwrap();

    // Test symlink creation
    let mut cmd = Command::cargo_bin("known").unwrap();
    cmd.current_dir(&project_dir)
        .env("HOME", temp_dir.path())
        .arg("symlink");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Successfully created symlinks"));

    // Verify that .rules directory structure is maintained
    assert!(project_dir.join(".rules").exists());
    assert!(project_dir.join(".rules").join("unix_rule.md").exists());
    assert!(project_dir.join(".rules").join("windows_rule.md").exists());

    // Verify symlinks point to correct content
    assert!(project_dir.join("CLAUDE.md").exists());
    assert!(project_dir.join("GEMINI.md").exists());
}

#[test]
fn test_symlink_overwrite_behavior() {
    let temp_dir = tempdir().unwrap();
    let project_dir = temp_dir.path().join("overwrite_test");
    std::fs::create_dir_all(&project_dir).unwrap();

    // Create initial AGENTS.md
    let initial_content = "# Initial Content\n\nThis is the initial content.";
    std::fs::write(project_dir.join("AGENTS.md"), initial_content).unwrap();

    // Create initial symlinks
    let mut cmd1 = Command::cargo_bin("known").unwrap();
    cmd1.current_dir(&project_dir)
        .env("HOME", temp_dir.path())
        .arg("symlink");
    cmd1.assert().success();

    // Verify initial symlinks
    let claude_content = std::fs::read_to_string(project_dir.join("CLAUDE.md")).unwrap();
    assert_eq!(claude_content, initial_content);

    // Update AGENTS.md content
    let updated_content = "# Updated Content\n\nThis is the updated content.";
    std::fs::write(project_dir.join("AGENTS.md"), updated_content).unwrap();

    // Create symlinks again (should overwrite)
    let mut cmd2 = Command::cargo_bin("known").unwrap();
    cmd2.current_dir(&project_dir)
        .env("HOME", temp_dir.path())
        .arg("symlink");
    cmd2.assert().success();

    // Verify symlinks were updated
    let updated_claude_content = std::fs::read_to_string(project_dir.join("CLAUDE.md")).unwrap();
    assert_eq!(updated_claude_content, updated_content);
}

#[test]
fn test_large_file_symlink_handling() {
    let temp_dir = tempdir().unwrap();
    let project_dir = temp_dir.path().join("large_file_test");
    std::fs::create_dir_all(&project_dir).unwrap();

    // Create a large AGENTS.md file (several KB)
    let mut large_content = String::new();
    large_content.push_str("# Large File Test\n\n");

    for i in 0..1000 {
        large_content.push_str(&format!("## Section {}\n\nThis is section {} with detailed content that makes the file larger. ", i, i));
        large_content.push_str("Lorem ipsum dolor sit amet, consectetur adipiscing elit. ");
        large_content
            .push_str("Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.\n\n");
    }

    std::fs::write(project_dir.join("AGENTS.md"), &large_content).unwrap();

    // Test symlink creation with large file
    let mut cmd = Command::cargo_bin("known").unwrap();
    cmd.current_dir(&project_dir)
        .env("HOME", temp_dir.path())
        .arg("symlink");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Successfully created symlinks"));

    // Verify large file was correctly symlinked
    let claude_content = std::fs::read_to_string(project_dir.join("CLAUDE.md")).unwrap();
    assert_eq!(claude_content, large_content);
    assert!(claude_content.len() > 10000); // Ensure it's actually large
}
