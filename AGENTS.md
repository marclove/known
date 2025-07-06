# AGENTS.md

This file provides guidance to agentic coding agents like [Claude Code](https://claude.ai/code), [Gemini CLI](https://github.com/google-gemini/gemini-cli), and [Codex CLI](https://github.com/openai/codex) when working with code in this repository. This is the place for documenting our product requirements and development guidelines.

## Project Overview

"known" is a Rust library that provides functionality for managing agentic LLM instruction files and project rules directories. It creates and manages AGENTS.md files with automatic migration from CLAUDE.md and GEMINI.md files, creates symlinks for compatibility, and manages rules directories for various AI coding assistants.

## Common Commands

- **Build**: `cargo build`
- **Run tests**: `cargo test`
- **Run specific test**: `cargo test test_name`
- **Check code**: `cargo check`
- **Format code**: `cargo fmt`
- **Lint**: `cargo clippy`

## Architecture

The project follows standard Rust library structure:
- `src/lib.rs` - Main library file containing public functions and tests
- `src/main.rs` - CLI interface using clap for command-line argument parsing
- `src/agents.rs` - AGENTS.md file creation and migration functionality
- `src/autostart.rs` - Cross-platform autostart management
- `src/config.rs` - Configuration file management for tracking watched directories
- `src/constants.rs` - Shared constants used throughout the application
- `src/daemon/` - Modular file watching daemon with single instance enforcement
  - `src/daemon/mod.rs` - Main daemon orchestration and lifecycle management
  - `src/daemon/events.rs` - File system event handling and processing
  - `src/daemon/watchers.rs` - File watcher setup and directory monitoring
  - `src/daemon/config_handler.rs` - Configuration file change handling and hot-reloading
  - `src/daemon/symlinks.rs` - Symlink management operations
- `src/single_instance.rs` - PID file locking for single instance enforcement
- `src/symlinks.rs` - Symlink creation and rules directory management
- `Cargo.toml` - Project configuration and dependencies

### Core Functions

The codebase provides the following main functionality:

1. **`create_agents_file()`** - Creates AGENTS.md files with case-insensitive checks and handles migration from existing CLAUDE.md or GEMINI.md files
2. **`create_symlinks()`** - Creates symlinks from CLAUDE.md and GEMINI.md to AGENTS.md, migrates files from .cursor/rules and .windsurf/rules to .rules directory, and automatically adds the directory to the configuration file for daemon tracking
3. **`start_daemon()`** - Starts a daemon that watches all configured directories from the configuration file, monitoring each directory's .rules subdirectory and maintaining synchronized symlinks across all projects. Enforces system-wide single instance operation using centralized PID file locking.
4. **`enable_autostart()`** - Enables cross-platform autostart for the daemon using the auto-launch crate
5. **`disable_autostart()`** - Disables autostart for the daemon
6. **`is_autostart_enabled()`** - Checks if autostart is currently enabled
7. **`SingleInstanceLock`** - Provides system-wide PID file locking mechanism to ensure only one daemon instance runs at a time across the entire system
8. **Configuration management functions**:
   - `load_config()` - Loads configuration from platform-specific application directory
   - `save_config()` - Saves configuration to platform-specific application directory
   - `add_directory_to_config()` - Adds a directory to the watched directories list
   - `remove_directory_from_config()` - Removes a directory from the watched directories list
9. **Daemon module functions**:
   - `run_daemon_event_loop()` - Main event processing loop for file system events
   - `handle_file_event()` - Handles individual file system events and updates symlinks accordingly
   - `setup_all_watchers()` - Sets up file watchers for all configured directories
   - `handle_config_file_change_internal()` - Handles configuration file changes and hot-reloading
   - `sync_rules_directory()` - Synchronizes existing files from .rules to target directories
   - `remove_symlinks_from_directory()` - Removes symlinks from target directories
10. **Helper functions**:
   - `ensure_rules_directory_exists()` - Creates .rules directory if it doesn't exist
   - `remove_existing_symlinks()` - Removes existing symlink files before creating new ones
   - `move_files_to_rules_dir()` - Moves files from source directories to .rules with conflict handling
   - `create_symlink_to_file()` - Creates platform-specific symlinks

### CLI Commands

- `known init` - Initialize project with AGENTS.md file and .rules directory
- `known symlink` - Create symlinks and migrate rules files from various AI assistant directories; automatically adds the directory to configuration for daemon tracking
- `known start` - Start daemon that watches all configured directories from the configuration file and maintains symlinks in .cursor/rules and .windsurf/rules
- `known run-daemon` - Internal command used by `start` to actually run the daemon process (users should use `start` instead)
- `known stop` - Stop the daemon process
- `known add [DIRECTORY]` - Add current working directory (or specified directory) to the list of watched directories
- `known remove [DIRECTORY]` - Remove current working directory (or specified directory) from the list of watched directories
- `known list` - List all directories currently being watched by the daemon
- `known enable-autostart` - Enable autostart for the daemon
- `known disable-autostart` - Disable autostart for the daemon
- `known autostart-status` - Check if autostart is enabled

All functions include comprehensive unit tests using Rust's built-in testing framework with the `tempfile` crate for file system isolation.

## Development Guidelines

1. Before implementing a new feature or trying to fix a bug, think deeply about your strategy first. Consider doing some web searches to clarify current best practices.
2. Write an acceptance test before writing your implementation code.
3. Verify that the test fails before writing your implementation.
4. Write the implementation code, always with proper docstrings.
5. Verify that the test now passes.
6. Check test coverage using `cargo tarpaulin` to ensure adequate test coverage is maintained.
7. Update the ROADMAP.md file.
8. Review the LESSONS_LEARNED.md file and consider making updates.
9. Review AGENTS.md and evaluate whether it should be updated.

### Backwards Compatibility Policy
- **Pre-Release Phase**: Since we have not done a public release yet, we do not need to provide backwards compatibility. Breaking changes can be implemented freely to improve the API and architecture without worrying about existing users.
- **Post-Release**: Once we have done our first public release (version 1.0.0), we will follow semantic versioning and provide appropriate backwards compatibility guarantees.

### Code Quality Standards
- **String Constants**: Extract repeated string literals into typed constants using `const NAME: &str = "value"` following `SCREAMING_SNAKE_CASE` naming conventions. This provides compile-time type checking and maintainability.

### Tool Usage Best Practices
- **Edit/Replace Tool Usage**: When using string replacement tools, always read the file first to see exact formatting (whitespace, line endings, indentation). Use the Read tool with specific line ranges or offsets to identify the precise text to replace. If replacement fails, examine the exact characters around the target text - often the issue is mismatched whitespace or line ending characters. For large functions, consider breaking changes into smaller, more targeted replacements rather than attempting to replace entire function bodies at once.

### Testing Standards
- **Integration Tests**: For CLI commands and end-to-end workflows, we use integration tests in the `tests` directory. These tests execute the compiled binary and assert its behavior, providing a robust safety net against regressions in user-facing functionality. Refer to our `TEST_COVERAGE.md` for our full testing coverage strategy.
- **Unit Tests**: Use unit tests for comprehensive testing of individual functions, testing private functions, edge cases, and complex logic validation. Unit tests should be isolated and use temporary directories for file system operations.
- **Doctests**: Only use doctests for documentation examples that users will copy-paste from your public API. Doctests should run by default to ensure documentation examples actually work.
- **Avoid `no_run` Doctests**: The `no_run` attribute should be rare and only used when examples can't run in the test environment (network access, user input, etc.). For file system operations with side effects, prefer unit tests over doctests.
- **File System Testing**: Use the `tempfile` crate for test isolation. Never use `std::env::set_current_dir()` in tests as it affects the entire process and can cause race conditions.

### Recordkeeping
- ROADMAP.md: After implementing a new feature, you MUST update this file.
- LESSONS_LEARNED.md: If you encounter difficulty with a particular bug that takes you several attempts to fix, you MUST update your notes for future reference so you don't get stuck on the same problem again.
- AGENTS.md: After making or changing any product or architecture decisions you MUST update AGENTS.md to maintain its accuracy. (This file is symlinked to CLAUDE.md and GEMINI.md so you only need to edit one file.)
- ADRs: We document all major architecture decisions in our ./adrs folder. When new decisions are made or we decide to change a major decision, we add a new ADR document in Markdown with numbered file names to maintain order of decisionmaking (e.g. ./adrs/001-our-first-decision.md)
