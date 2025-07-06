# Test Coverage Improvement Plan

Current test coverage is **49.67%**. This plan outlines an iterative strategy to increase test coverage in a meaningful way, focusing on ensuring the reliability and stability of the `known` CLI and daemon.

## Guiding Principles

1.  **Meaningful Testing**: We will prioritize tests that cover critical logic, common use cases, and essential error-handling paths. The goal is not just to reach a high percentage, but to build a truly robust test suite.
2.  **Avoid Brittle Tests**: Tests should not be tightly coupled to implementation details that are likely to change. We will favor testing public APIs and observable behavior over internal state.
3.  **Integration Tests for CLI**: The behavior of the main binary (`src/main.rs`) will be tested using integration tests that execute the compiled application. This is the most effective way to validate command-line argument parsing, command dispatch, and end-to-end functionality.
4.  **Focus on Error Handling**: A significant portion of the uncovered code lies in `if let Err(e) = ...` blocks. We will add tests that specifically trigger these error conditions to ensure the application behaves gracefully under failure scenarios.

## Iterative Plan

This plan is broken down into phases, starting with the area that will provide the most impact.

### Phase 1: CLI Integration Testing (`src/main.rs`)

**Goal**: Achieve high test coverage for the CLI entry point and command-line interface. This is the highest priority as it currently has 0% coverage.

**Steps**:

1.  **Add Testing Dependencies**: Introduce `assert_cmd` and `predicates` as new development dependencies in `Cargo.toml` to facilitate CLI testing.
2.  **Create Integration Test Suite**: Create a new test file, `tests/cli.rs`.
3.  **Test Command Success Cases**:
    *   Write a test for `known init` and assert that `AGENTS.md` and `.rules` are created.
    *   Write tests for `known add <dir>` and `known remove <dir>` and assert that the configuration file is updated correctly.
    *   Write a test for `known symlink` and verify that symlinks are created as expected.
4.  **Test Command Failure Cases**:
    *   Write tests that run commands with invalid arguments (e.g., `known add` with no directory) and assert that the correct error messages and exit codes are produced.
    *   Test help messages (`known --help`, `known init --help`, etc.).
5.  **Test Daemon Commands**:
    *   Write tests for `known start`, `known stop`, and `known status`. These tests will need to manage a daemon process, likely using `std::process::Command` to run the binary in the background and then interact with it.

### Phase 2: Daemon and Configuration (`src/daemon.rs`, `src/config.rs`)

**Goal**: Harden the daemon's core logic by testing its interaction with the configuration and the file system.

**Steps**:

1.  **Configuration I/O Errors**:
    *   In `src/config.rs`, write tests for `load_config` that handle a corrupted or malformed JSON file.
    *   Write tests for `save_config` where file write permissions are denied. This can be simulated in a test by creating a file and then setting its permissions to read-only using `std::fs::set_permissions`.
2.  **Daemon Startup and Watcher Logic**:
    *   In `src/daemon.rs`, test the `start_daemon` function's behavior when the configuration file is missing or invalid.
    *   Expand tests for `handle_file_event` to simulate more `notify::EventKind` variations (e.g., `Modify(ModifyKind::Name(RenameMode::Any))`) to ensure symlinks and directories are handled correctly when renamed.
    *   Test the scenario where a directory being watched by the daemon is deleted.

### Phase 3: Autostart and Single Instance (`src/autostart.rs`, `src/single_instance.rs`)

**Goal**: Improve coverage for modules that interact heavily with the operating system, focusing on their error-handling capabilities.

**Steps**:

1.  **Single Instance Lock Failures**:
    *   In `src/single_instance.rs`, write a test for `SingleInstanceLock::new` that simulates a failure to create the lock directory due to permissions.
    *   Write tests for `stop_daemon` that cover edge cases like a malformed PID file (e.g., it contains non-numeric text or is empty).
2.  **Autostart Error Handling**:
    *   Testing the `autostart` module is challenging without mocking the `auto-launch` crate. The immediate goal is to ensure all error-handling branches (the `if let Err(e) = ...` blocks) are at least exercised by tests, even if the underlying error condition cannot be perfectly simulated in a unit test.
    *   A long-term strategy could involve introducing a feature flag to compile in a mock version of the autostart functionality for testing purposes. For now, we will acknowledge this limitation.

### Phase 4: High-Coverage Polish (`src/agents.rs`, `src/symlinks.rs`)

**Goal**: Bring the remaining, already well-tested modules as close to 100% coverage as is practical.

**Steps**:

1.  **File Operation Failures**:
    *   In `src/agents.rs` and `src/symlinks.rs`, identify the remaining uncovered lines, which are primarily error-handling paths for file system operations.
    *   Write tests that trigger these I/O errors. For example, attempt to create a symlink in a directory where the test does not have write permissions to trigger and verify the `Err` result.
