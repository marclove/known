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
