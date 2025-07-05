# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust library project named "known" using Rust 2021 edition. The library provides functionality for creating an empty AGENTS.md file in the current working directory if it doesn't already exist (case-insensitive check).

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
- `Cargo.toml` - Project configuration and dependencies

The codebase currently contains a `create_agents_file()` function that checks for existing agents.md files (case-insensitive) and creates an empty AGENTS.md file if none exists. The function includes comprehensive unit tests using Rust's built-in testing framework.