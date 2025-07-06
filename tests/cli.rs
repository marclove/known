use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_init_command() {
    let mut cmd = Command::cargo_bin("known").unwrap();
    let temp_dir = tempdir().unwrap();

    cmd.current_dir(temp_dir.path()).arg("init");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(
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

    // Test `add` command
    cmd.current_dir(&project_dir)
        .env("HOME", temp_dir.path())
        .arg("add");

    cmd.assert().success().stdout(predicate::str::contains(
        "Successfully added",
    ));
}

#[test]
fn test_remove_command() {
    let mut cmd = Command::cargo_bin("known").unwrap();
    let temp_dir = tempdir().unwrap();
    let project_dir = temp_dir.path().join("project");
    std::fs::create_dir(&project_dir).unwrap();

    // First, add the directory
    let mut add_cmd = Command::cargo_bin("known").unwrap();
    add_cmd
        .current_dir(&project_dir)
        .env("HOME", temp_dir.path())
        .arg("add");
    add_cmd.assert().success();

    // Then, test the `remove` command
    cmd.current_dir(&project_dir)
        .env("HOME", temp_dir.path())
        .arg("remove");

    cmd.assert().success().stdout(predicate::str::contains(
        "Successfully removed",
    ));
}

#[test]
fn test_symlink_command() {
    let mut cmd = Command::cargo_bin("known").unwrap();
    let temp_dir = tempdir().unwrap();
    let project_dir = temp_dir.path().join("project");
    std::fs::create_dir(&project_dir).unwrap();
    std::fs::write(project_dir.join("AGENTS.md"), "test content").unwrap();

    cmd.current_dir(&project_dir)
        .env("HOME", temp_dir.path())
        .arg("symlink");

    cmd.assert().success().stdout(predicate::str::contains(
        "Successfully created symlinks",
    ));

    assert!(project_dir.join("CLAUDE.md").exists());
    assert!(project_dir.join("GEMINI.md").exists());
}
