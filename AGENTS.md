# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository. This is the place for documenting our product requirements and development guidelines.

## Project Overview

This is a Rust library project named "known" using Rust 2021 edition. The library provides functionality for managing agentic LLM instruction files and project rules directories. It creates and manages AGENTS.md files with automatic migration from CLAUDE.md and GEMINI.md files, creates symlinks for compatibility, and manages rules directories for various AI coding assistants.

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
- `Cargo.toml` - Project configuration and dependencies

### Core Functions

The codebase provides the following main functionality:

1. **`create_agents_file()`** - Creates AGENTS.md files with case-insensitive checks and handles migration from existing CLAUDE.md or GEMINI.md files
2. **`create_symlinks()`** - Creates symlinks from CLAUDE.md and GEMINI.md to AGENTS.md, and migrates files from .cursor/rules and .windsurf/rules to .rules directory
3. **`start_daemon()`** - Starts a file watching daemon that monitors .rules directory and maintains synchronized symlinks in .cursor/rules and .windsurf/rules
4. **Helper functions**:
   - `ensure_rules_directory_exists()` - Creates .rules directory if it doesn't exist
   - `remove_existing_symlinks()` - Removes existing symlink files before creating new ones
   - `move_files_to_rules_dir()` - Moves files from source directories to .rules with conflict handling
   - `handle_file_event()` - Handles file system events and updates symlinks accordingly
   - `create_symlink_to_file()` - Creates platform-specific symlinks
   - `sync_rules_directory()` - Synchronizes existing files from .rules to target directories

### CLI Commands

- `known init` - Initialize project with AGENTS.md file and .rules directory
- `known symlink` - Create symlinks and migrate rules files from various AI assistant directories
- `known daemon` - Start daemon to watch .rules directory and maintain symlinks in .cursor/rules and .windsurf/rules

All functions include comprehensive unit tests using Rust's built-in testing framework with the `tempfile` crate for file system isolation.

## Development Guidelines

1. Before implementing a new feature or trying to fix a bug, think deeply about your strategy first. Consider doing some web searches to clarify current best practices.
2. Write an acceptance test before writing your implementation code.
3. Verify that the test fails before writing your implementation.
4. Write the implementation code, always with proper docstrings.
5. Verify that the test now passes.
6. Update the ROADMAP.md file.
7. Review the LESSONS_LEARNED.md file and consider making updates.
8. Review CLAUDE.md and evaluate whether it should be updated.

### Code Quality Standards
- **String Constants**: Extract repeated string literals into typed constants using `const NAME: &str = "value"` following `SCREAMING_SNAKE_CASE` naming conventions. This provides compile-time type checking and maintainability.

### Testing Standards
- **Unit Tests**: Use unit tests for comprehensive testing of individual functions, testing private functions, edge cases, and complex logic validation. Unit tests should be isolated and use temporary directories for file system operations.
- **Doctests**: Only use doctests for documentation examples that users will copy-paste from your public API. Doctests should run by default to ensure documentation examples actually work.
- **Avoid `no_run` Doctests**: The `no_run` attribute should be rare and only used when examples can't run in the test environment (network access, user input, etc.). For file system operations with side effects, prefer unit tests over doctests.
- **File System Testing**: Use the `tempfile` crate for test isolation. Never use `std::env::set_current_dir()` in tests as it affects the entire process and can cause race conditions.

### Recordkeeping
- ROADMAP.md: After implementing a new feature, you MUST update this file.
- LESSONS_LEARNED.md: If you encounter difficulty with a particular bug that takes you several attempts to fix, you MUST update your notes for future reference so you don't get stuck on the same problem again.
- CLAUDE.md: After making or changing any product or architecture decisions you MUST update CLAUDE.md to maintain its accuracy.
- ADRs: We document all major architecture decisions in our ./adrs folder. When new decisions are made or we decide to change a major decision, we add a new ADR document in Markdown with numbered file names to maintain order of decisionmaking (e.g. ./adrs/001-our-first-decision.md)
