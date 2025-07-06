use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_init_command() {
    let mut cmd = Command::cargo_bin("known").unwrap();
    let temp_dir = tempdir().unwrap();

    cmd.current_dir(temp_dir.path()).arg("init");

    cmd.assert().success().stdout(predicate::str::contains(
        "Successfully initialized project with AGENTS.md",
    ));

    assert!(temp_dir.path().join("AGENTS.md").exists());
    assert!(temp_dir.path().join(".rules").exists());
}

#[test]
fn test_add_command() {
    let mut cmd = Command::cargo_bin("known").unwrap();
    let temp_dir = tempdir().unwrap();
    let project_dir = temp_dir.path().join("project");
    std::fs::create_dir(&project_dir).unwrap();

    // Create the config directory structure that the directories crate expects
    let config_dir = temp_dir.path().join(".config").join("known");
    std::fs::create_dir_all(&config_dir).unwrap();

    // Test `add` command
    cmd.env("HOME", temp_dir.path())
        .arg("add")
        .arg(&project_dir);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Successfully added"));
}

#[test]
fn test_remove_command() {
    let mut cmd = Command::cargo_bin("known").unwrap();
    let temp_dir = tempdir().unwrap();
    let project_dir = temp_dir.path().join("project");
    std::fs::create_dir(&project_dir).unwrap();

    // Create the config directory structure that the directories crate expects
    let config_dir = temp_dir.path().join(".config").join("known");
    std::fs::create_dir_all(&config_dir).unwrap();

    // Verify project directory exists before testing
    assert!(
        project_dir.exists(),
        "Project directory should exist before running add command"
    );

    // First, add the directory
    let mut add_cmd = Command::cargo_bin("known").unwrap();
    add_cmd
        .env("HOME", temp_dir.path())
        .arg("add")
        .arg(&project_dir);
    add_cmd.assert().success();

    // Then, test the `remove` command
    cmd.env("HOME", temp_dir.path())
        .arg("remove")
        .arg(&project_dir);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Successfully removed"));
}

#[test]
fn test_symlink_command() {
    let mut cmd = Command::cargo_bin("known").unwrap();
    let temp_dir = tempdir().unwrap();
    let project_dir = temp_dir.path().join("project");
    std::fs::create_dir(&project_dir).unwrap();
    std::fs::write(project_dir.join("AGENTS.md"), "test content").unwrap();

    // Create the config directory structure that the directories crate expects
    let config_dir = temp_dir.path().join(".config").join("known");
    std::fs::create_dir_all(&config_dir).unwrap();

    cmd.current_dir(&project_dir)
        .env("HOME", temp_dir.path())
        .arg("symlink");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Successfully created symlinks"));

    assert!(project_dir.join("CLAUDE.md").exists());
    assert!(project_dir.join("GEMINI.md").exists());
}

#[test]
fn test_help_messages() {
    let mut cmd = Command::cargo_bin("known").unwrap();
    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains(
            "A CLI tool for managing project files",
        ))
        .stdout(predicate::str::contains("Usage: known <COMMAND>"))
        .stdout(predicate::str::contains("Commands:"))
        .stdout(predicate::str::contains("init"))
        .stdout(predicate::str::contains("symlink"))
        .stdout(predicate::str::contains("add"))
        .stdout(predicate::str::contains("remove"));

    let mut cmd = Command::cargo_bin("known").unwrap();
    cmd.arg("init").arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Usage: known init"));

    let mut cmd = Command::cargo_bin("known").unwrap();
    cmd.arg("add").arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Usage: known add"));

    let mut cmd = Command::cargo_bin("known").unwrap();
    cmd.arg("remove").arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Usage: known remove"));

    let mut cmd = Command::cargo_bin("known").unwrap();
    cmd.arg("symlink").arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Usage: known symlink"));
}
