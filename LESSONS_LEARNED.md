# Lessons Learned

## Testing File System Operations
- Use `tempfile` crate for isolated temporary directories instead of changing current directory
- Refactor functions to accept directory parameters for better testability
- Avoid `std::env::set_current_dir()` in tests - it affects the entire process and can cause race conditions
- Use absolute paths constructed from temporary directory paths instead of relative paths

## Claude Code Directory Access Restrictions
- Claude Code blocks access to `/tmp` and other system directories for security reasons
- When manually testing CLI functionality, stay within the project's working directory structure
- Unit tests using `tempfile::tempdir()` work correctly - they create temp directories in accessible locations
- Don't confuse manual CLI testing limitations with unit test capabilities - they use different mechanisms

## File System Watching with notify Crate
- On macOS, file system events may return paths with `/private` prefix while the watched directory path doesn't have this prefix
- Always canonicalize paths before comparing them in file watchers to handle symlinks and path resolution differences
- The `notify` crate requires proper path matching - use `path.starts_with(canonical_path)` for reliable event filtering
- File creation events often generate multiple events (Create, Modify metadata, Modify content) - handle all relevant event types
- Use `RecursiveMode::NonRecursive` when only watching direct files in a directory, not subdirectories
- **File Rename Handling**: `EventKind::Modify(_)` is NOT the same as `EventKind::Create(_)` - file renames generate two separate events: `Modify(ModifyKind::Name(RenameMode::From))` for the old name and `Modify(ModifyKind::Name(RenameMode::To))` for the new name. Handle them differently: remove symlinks for `From` events and create symlinks for `To` events to avoid stale symlinks

## Testing System Process Operations
- **Signal Interference in Parallel Tests**: Tests that send real signals (like SIGTERM) to processes can interfere with other tests running in parallel, causing failures like "signal: 15, SIGTERM: termination signal"
- **Isolation Pattern**: For functions that interact with system processes, provide test-specific versions that accept custom file paths instead of using system-wide paths. Example: `stop_daemon_with_test_path()` vs `stop_daemon()`
- **Test Separation**: Keep acceptance tests (using real system functions) separate from unit tests (using isolated test functions) to prevent cross-contamination
- **Test Naming Convention**: Use clear naming to distinguish between isolated unit tests and system integration tests (e.g., `test_stop_daemon_no_pid_file` vs `test_stop_command_acceptance`)
- **Multiple Test Verification**: Run tests multiple times consecutively to verify stability after fixing parallel execution issues
- **Global State Isolation**: Tests that modify global state (like configuration files) should use isolated test-specific functions. Create variants that accept parameters (config, file paths) instead of reading from global state. Example: instead of using `add_directory_to_config()` which modifies the system config, create `add_directory_to_config_file(dir_path, config_path)` for tests to use with temporary config files
- **Lock Contention**: System-wide locks (like `SingleInstanceLock::acquire()`) can cause parallel test failures. Provide lock-free test variants for unit testing the core logic without the locking mechanism
- **Eliminating serial_test**: By properly isolating global state access, tests can run in parallel without `#[serial_test::serial]` annotations, improving test performance and reducing dependencies
