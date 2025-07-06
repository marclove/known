//! A Rust library for managing project Agentic LLM instruction files.
//!
//! This library provides functionality for creating and managing AGENTS.md files
//! in project directories, with support for renaming existing CLAUDE.md files.

pub mod agents;
pub mod daemon;
pub mod symlinks;

// Re-export public API functions
pub use agents::{create_agents_file, create_agents_file_in_dir};
pub use daemon::start_daemon;
pub use symlinks::{create_symlinks, create_symlinks_in_dir};