//! A Rust library for managing project Agentic LLM instruction files.
//!
//! This library provides functionality for creating and managing AGENTS.md files
//! in project directories, with support for renaming existing CLAUDE.md files.

pub mod agents;
pub mod autostart;
pub mod config;
pub mod constants;
pub mod daemon;
pub mod single_instance;
pub mod symlinks;

// Re-export public API functions
pub use agents::{create_agents_file, create_agents_file_in_dir};
pub use autostart::{disable_autostart, enable_autostart, is_autostart_enabled};
pub use config::{
    add_directory_to_config, add_directory_to_config_file, load_config, load_config_from_file,
    remove_directory_from_config, remove_directory_from_config_file, save_config, Config,
};
pub use daemon::start_daemon;
pub use single_instance::{stop_daemon, SingleInstanceLock};
pub use symlinks::{create_symlinks, create_symlinks_in_dir};
