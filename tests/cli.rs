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
    let temp_dir = tempdir().unwrap();
    let project_dir = temp_dir.path().join("project");
    std::fs::create_dir(&project_dir).unwrap();

    // Determine what config directory the directories crate would actually use
    // with our custom HOME environment
    let config_dir = if cfg!(target_os = "macos") {
        temp_dir
            .path()
            .join("Library")
            .join("Application Support")
            .join("known")
    } else {
        temp_dir.path().join(".config").join("known")
    };
    std::fs::create_dir_all(&config_dir).unwrap();

    // Log environment details for CI debugging
    println!("Test environment:");
    println!("  temp_dir: {}", temp_dir.path().display());
    println!("  project_dir: {}", project_dir.display());
    println!("  config_dir: {}", config_dir.display());
    println!("  project_dir.exists(): {}", project_dir.exists());
    println!("  config_dir.exists(): {}", config_dir.exists());
    println!("  temp_dir.path().exists(): {}", temp_dir.path().exists());

    // Verify directories exist before running command
    assert!(project_dir.exists(), "Project directory should exist");
    assert!(config_dir.exists(), "Config directory should exist");

    // Test `add` command
    let mut cmd = Command::cargo_bin("known").unwrap();
    cmd.env("HOME", temp_dir.path())
        .env("CI", "1") // Force CI environment for debug logging
        .arg("add")
        .arg(&project_dir);

    println!("Running command: known add {}", project_dir.display());
    println!("With HOME={}", temp_dir.path().display());

    // Check what config path would be resolved with our HOME override
    println!(
        "Expected config path: {}",
        config_dir.join("config.json").display()
    );

    let result = cmd.assert();

    // Log any stderr for debugging
    println!("Command completed, checking output...");

    result
        .success()
        .stdout(predicate::str::contains("Successfully added"));

    println!("test_add_command completed successfully");
}

#[test]
fn test_remove_command() {
    let temp_dir = tempdir().unwrap();
    let project_dir = temp_dir.path().join("project");
    std::fs::create_dir(&project_dir).unwrap();

    // Determine what config directory the directories crate would actually use
    // with our custom HOME environment
    let config_dir = if cfg!(target_os = "macos") {
        temp_dir
            .path()
            .join("Library")
            .join("Application Support")
            .join("known")
    } else {
        temp_dir.path().join(".config").join("known")
    };
    std::fs::create_dir_all(&config_dir).unwrap();

    // Log environment details for CI debugging
    println!("Test environment:");
    println!("  temp_dir: {}", temp_dir.path().display());
    println!("  project_dir: {}", project_dir.display());
    println!("  config_dir: {}", config_dir.display());
    println!("  project_dir.exists(): {}", project_dir.exists());
    println!("  config_dir.exists(): {}", config_dir.exists());

    // Verify directories exist before testing
    assert!(project_dir.exists(), "Project directory should exist");
    assert!(config_dir.exists(), "Config directory should exist");

    // First, add the directory
    println!("Running add command: known add {}", project_dir.display());
    let mut add_cmd = Command::cargo_bin("known").unwrap();
    add_cmd
        .env("HOME", temp_dir.path())
        .env("CI", "1") // Force CI environment for debug logging
        .arg("add")
        .arg(&project_dir);

    add_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains("Successfully added"));
    println!("Add command completed successfully");

    // Then, test the `remove` command
    println!(
        "Running remove command: known remove {}",
        project_dir.display()
    );
    let mut remove_cmd = Command::cargo_bin("known").unwrap();
    remove_cmd
        .env("HOME", temp_dir.path())
        .env("CI", "1") // Force CI environment for debug logging
        .arg("remove")
        .arg(&project_dir);

    remove_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains("Successfully removed"));

    println!("test_remove_command completed successfully");
}

#[test]
fn test_symlink_command() {
    let mut cmd = Command::cargo_bin("known").unwrap();
    let temp_dir = tempdir().unwrap();
    let project_dir = temp_dir.path().join("project");
    std::fs::create_dir(&project_dir).unwrap();
    std::fs::write(project_dir.join("AGENTS.md"), "test content").unwrap();

    // Determine what config directory the directories crate would actually use
    // with our custom HOME environment
    let config_dir = if cfg!(target_os = "macos") {
        temp_dir
            .path()
            .join("Library")
            .join("Application Support")
            .join("known")
    } else {
        temp_dir.path().join(".config").join("known")
    };
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
#[cfg_attr(ci, ignore)]
fn test_autostart_commands() {
    let mut cmd = Command::cargo_bin("known").unwrap();
    cmd.arg("enable-autostart");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Autostart enabled successfully"));

    let mut cmd = Command::cargo_bin("known").unwrap();
    cmd.arg("autostart-status");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Autostart is enabled"));

    let mut cmd = Command::cargo_bin("known").unwrap();
    cmd.arg("disable-autostart");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Autostart disabled successfully"));

    let mut cmd = Command::cargo_bin("known").unwrap();
    cmd.arg("autostart-status");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Autostart is disabled"));
}

#[test]
fn test_stop_command() {
    let mut cmd = Command::cargo_bin("known").unwrap();
    cmd.arg("stop");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("No daemon is currently running"));
}

#[test]
fn test_list_command() {
    let temp_dir = tempdir().unwrap();
    let project_dir = temp_dir.path().join("project");
    std::fs::create_dir(&project_dir).unwrap();

    let config_dir = if cfg!(target_os = "macos") {
        temp_dir
            .path()
            .join("Library")
            .join("Application Support")
            .join("known")
    } else {
        temp_dir.path().join(".config").join("known")
    };
    std::fs::create_dir_all(&config_dir).unwrap();

    let mut cmd = Command::cargo_bin("known").unwrap();
    cmd.env("HOME", temp_dir.path()).arg("list");
    cmd.assert().success().stdout(predicate::str::contains(
        "No directories are currently being watched",
    ));

    let mut add_cmd = Command::cargo_bin("known").unwrap();
    add_cmd
        .env("HOME", temp_dir.path())
        .arg("add")
        .arg(&project_dir);
    add_cmd.assert().success();

    let mut cmd = Command::cargo_bin("known").unwrap();
    cmd.env("HOME", temp_dir.path()).arg("list");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Watched directories:"))
        .stdout(predicate::str::contains(project_dir.to_str().unwrap()));
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
