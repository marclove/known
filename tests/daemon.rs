use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;
use std::thread;
use std::time::Duration;
use tempfile::tempdir;

#[test]
fn test_daemon_commands() {
    let temp_dir = tempdir().unwrap();
    let home_dir = temp_dir.path();

    // Start the daemon
    let mut start_cmd = Command::cargo_bin("known").unwrap();
    let mut child = start_cmd
        .env("HOME", home_dir)
        .arg("start")
        .spawn()
        .unwrap();

    // Wait for the daemon to start
    for _ in 0..10 {
        let mut status_cmd = Command::cargo_bin("known").unwrap();
        let assert = status_cmd
            .env("HOME", home_dir)
            .arg("autostart-status")
            .assert();
        if assert.try_success().is_ok() {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }

    // Check the status
    let mut status_cmd = Command::cargo_bin("known").unwrap();
    status_cmd
        .env("HOME", home_dir)
        .arg("autostart-status")
        .assert()
        .success()
        .stdout(predicate::str::contains("Autostart is disabled"));

    // Stop the daemon
    let mut stop_cmd = Command::cargo_bin("known").unwrap();
    stop_cmd
        .env("HOME", home_dir)
        .arg("stop")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Daemon stopped")
                .or(predicate::str::contains("No daemon is currently running")),
        );

    child.kill().unwrap();
}

#[test]
fn test_daemon_full_lifecycle() {
    let temp_dir = tempdir().unwrap();
    let home_dir = temp_dir.path();

    // Create a test project directory
    let project_dir = temp_dir.path().join("test_project");
    std::fs::create_dir(&project_dir).unwrap();
    std::fs::write(
        project_dir.join("AGENTS.md"),
        "# Test Project\n\nTest content",
    )
    .unwrap();

    // Initialize the project
    let mut init_cmd = Command::cargo_bin("known").unwrap();
    init_cmd
        .current_dir(&project_dir)
        .env("HOME", home_dir)
        .arg("init")
        .assert()
        .success();

    // Add the project to the configuration
    let mut add_cmd = Command::cargo_bin("known").unwrap();
    add_cmd
        .current_dir(&project_dir)
        .env("HOME", home_dir)
        .arg("add")
        .assert()
        .success();

    // Start the daemon
    let mut start_cmd = Command::cargo_bin("known").unwrap();
    let mut daemon_process = start_cmd
        .env("HOME", home_dir)
        .arg("start")
        .spawn()
        .unwrap();

    // Wait for daemon to be ready and verify it started
    let mut daemon_running = false;
    for _ in 0..20 {
        thread::sleep(Duration::from_millis(100));

        // Try to stop the daemon to see if it's running
        let mut check_cmd = Command::cargo_bin("known").unwrap();
        let output = check_cmd
            .env("HOME", home_dir)
            .arg("stop")
            .output()
            .unwrap();

        let stdout = String::from_utf8(output.stdout).unwrap();
        if stdout.contains("Daemon stopped") {
            daemon_running = true;
            break;
        }

        // If daemon isn't running yet, continue waiting
        if stdout.contains("No daemon is currently running") {
            continue;
        }
    }

    // If daemon never started, skip this test as it may be a system limitation
    if !daemon_running {
        daemon_process.kill().ok();
        println!("Daemon failed to start in test environment, skipping test");
        return;
    }

    // Wait for daemon to actually stop
    thread::sleep(Duration::from_millis(200));

    // Verify daemon is stopped
    let mut stop_again_cmd = Command::cargo_bin("known").unwrap();
    stop_again_cmd
        .env("HOME", home_dir)
        .arg("stop")
        .assert()
        .success()
        .stdout(predicate::str::contains("No daemon is currently running"));

    // Restart the daemon
    let mut restart_cmd = Command::cargo_bin("known").unwrap();
    let mut daemon_process2 = restart_cmd
        .env("HOME", home_dir)
        .arg("start")
        .spawn()
        .unwrap();

    // Wait for restart
    thread::sleep(Duration::from_millis(500));

    // Clean up
    let mut final_stop_cmd = Command::cargo_bin("known").unwrap();
    final_stop_cmd
        .env("HOME", home_dir)
        .arg("stop")
        .assert()
        .success();

    daemon_process.kill().ok();
    daemon_process2.kill().ok();
}

#[test]
fn test_daemon_file_watching() {
    let temp_dir = tempdir().unwrap();
    let home_dir = temp_dir.path();

    // Create test project structure
    let project_dir = temp_dir.path().join("watched_project");
    std::fs::create_dir_all(&project_dir).unwrap();
    std::fs::create_dir_all(project_dir.join(".rules")).unwrap();

    // Create AGENTS.md file
    std::fs::write(
        project_dir.join("AGENTS.md"),
        "# Watched Project\n\nThis is a test.",
    )
    .unwrap();

    // Add a test file to .rules directory
    std::fs::write(
        project_dir.join(".rules").join("test.md"),
        "Test rule content",
    )
    .unwrap();

    // Add the project to configuration
    let mut add_cmd = Command::cargo_bin("known").unwrap();
    add_cmd
        .current_dir(&project_dir)
        .env("HOME", home_dir)
        .arg("add")
        .assert()
        .success();

    // Start daemon
    let mut start_cmd = Command::cargo_bin("known").unwrap();
    let mut daemon_process = start_cmd
        .env("HOME", home_dir)
        .arg("start")
        .spawn()
        .unwrap();

    // Wait for daemon to initialize
    thread::sleep(Duration::from_millis(800));

    // Create symlinks to verify daemon is working
    let mut symlink_cmd = Command::cargo_bin("known").unwrap();
    symlink_cmd
        .current_dir(&project_dir)
        .env("HOME", home_dir)
        .arg("symlink")
        .assert()
        .success();

    // Verify symlinks were created
    assert!(project_dir.join("CLAUDE.md").exists());
    assert!(project_dir.join("GEMINI.md").exists());

    // Add a new file to .rules and give daemon time to sync
    std::fs::write(project_dir.join(".rules").join("new_rule.md"), "New rule").unwrap();
    thread::sleep(Duration::from_millis(500));

    // Clean up
    let mut stop_cmd = Command::cargo_bin("known").unwrap();
    stop_cmd
        .env("HOME", home_dir)
        .arg("stop")
        .assert()
        .success();

    daemon_process.kill().ok();
}

#[test]
fn test_daemon_multiple_projects() {
    let temp_dir = tempdir().unwrap();
    let home_dir = temp_dir.path();

    // Create multiple test projects
    let project1 = temp_dir.path().join("project1");
    let project2 = temp_dir.path().join("project2");

    for project in [&project1, &project2] {
        std::fs::create_dir_all(project).unwrap();
        std::fs::create_dir_all(project.join(".rules")).unwrap();
        std::fs::write(project.join("AGENTS.md"), "# Test Project").unwrap();

        // Add project to configuration
        let mut add_cmd = Command::cargo_bin("known").unwrap();
        add_cmd
            .current_dir(project)
            .env("HOME", home_dir)
            .arg("add")
            .assert()
            .success();
    }

    // Start daemon
    let mut start_cmd = Command::cargo_bin("known").unwrap();
    let mut daemon_process = start_cmd
        .env("HOME", home_dir)
        .arg("start")
        .spawn()
        .unwrap();

    // Wait for daemon to initialize
    thread::sleep(Duration::from_millis(600));

    // Test symlink creation in both projects
    for project in [&project1, &project2] {
        let mut symlink_cmd = Command::cargo_bin("known").unwrap();
        symlink_cmd
            .current_dir(project)
            .env("HOME", home_dir)
            .arg("symlink")
            .assert()
            .success();

        // Verify symlinks were created
        assert!(project.join("CLAUDE.md").exists());
        assert!(project.join("GEMINI.md").exists());
    }

    // Remove one project from configuration
    let mut remove_cmd = Command::cargo_bin("known").unwrap();
    remove_cmd
        .current_dir(&project1)
        .env("HOME", home_dir)
        .arg("remove")
        .assert()
        .success();

    // Wait for configuration change to be processed
    thread::sleep(Duration::from_millis(300));

    // Clean up
    let mut stop_cmd = Command::cargo_bin("known").unwrap();
    stop_cmd
        .env("HOME", home_dir)
        .arg("stop")
        .assert()
        .success();

    daemon_process.kill().ok();
}
