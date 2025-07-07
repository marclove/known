# Known

A Rust library and CLI tool for managing agentic LLM instruction files in your projects.

## Overview

Known helps you create and manage instruction files for various AI coding assistants like Claude Code, Gemini CLI, and other agentic tools. It provides a unified `AGENTS.md` file format with automatic symlink generation for compatibility with different naming conventions.

## Features

- **Unified instruction file**: Creates `AGENTS.md` as the single source of truth
- **Automatic migration**: Renames existing `CLAUDE.md` or `GEMINI.md` files to `AGENTS.md`
- **Symlink generation**: Creates `CLAUDE.md` and `GEMINI.md` symlinks pointing to `AGENTS.md`
- **Rules directory management**: Automatically creates `.rules` directory and migrates files from `.cursor/rules` and `.windsurf/rules`
- **Daemon process**: File watching daemon that maintains synchronized symlinks across IDE rules directories
- **System-wide single instance enforcement**: Prevents multiple daemon instances from running simultaneously across the entire system using centralized PID file locking
- **Cross-platform autostart**: System-level autostart configuration for seamless daemon management
- **Cross-platform compatibility**: Works on Unix and Windows systems
- **CLI interface**: Simple command-line tool for project initialization and management

## Installation

```bash
cargo install known
```

## Usage

### Initialize a project

Create an `AGENTS.md` file in your current directory:

```bash
known init
```

This command will:
- Create an `AGENTS.md` file with default content if none exists
- Rename existing `CLAUDE.md` or `GEMINI.md` files to `AGENTS.md`
- Handle conflicts gracefully when multiple instruction files exist
- Create a `.rules` directory for storing project-specific rules

### Create symlinks

Generate compatibility symlinks for different AI tools:

```bash
known symlink
```

This command will:
- Create `CLAUDE.md` → `AGENTS.md` (symlink)
- Create `GEMINI.md` → `AGENTS.md` (symlink)
- Move any files from `.cursor/rules` to `.rules` directory
- Move any files from `.windsurf/rules` to `.rules` directory
- Skip files that already exist in `.rules` with a user-friendly warning

### Manage watched directories

Add a directory to be watched by the daemon:

```bash
known add [DIRECTORY]
```

If no directory is specified, the current working directory is used. This command will:
- Add the specified directory to the daemon's configuration
- Enable the daemon to watch the `.rules` directory within that project

Remove a directory from being watched:

```bash
known remove [DIRECTORY]
```

If no directory is specified, the current working directory is used.

List all directories currently being watched:

```bash
known list
```

This command displays all directories that are currently configured to be watched by the daemon.

### Start daemon

Start a file watching daemon to automatically maintain symlinks:

```bash
known start
```

This command will:
- Monitor all configured directories' `.rules` subdirectories for changes
- Automatically create and maintain symlinks in `.cursor/rules` and `.windsurf/rules`
- Keep the rules directories synchronized with the unified `.rules` directory
- Enforce system-wide single instance operation (only one daemon can run across the entire system)
- Create a centralized PID file for process management
- Run continuously until stopped, even if no directories are initially configured

Stop the daemon:

```bash
known stop
```

### Autostart management

Enable the daemon to start automatically when your system boots:

```bash
known enable-autostart
```

Disable autostart for the daemon:

```bash
known disable-autostart
```

Check if autostart is enabled:

```bash
known autostart-status
```

The autostart feature works cross-platform:
- **Windows**: Uses Windows registry entries
- **macOS**: Uses Launch Agents 
- **Linux**: Uses systemd or equivalent service manager

## Library Usage

You can also use Known as a Rust library:

```rust
use known::{create_agents_file, create_symlinks, start_daemon, enable_autostart, disable_autostart, is_autostart_enabled, SingleInstanceLock};

// Create AGENTS.md file
create_agents_file()?;

// Create symlinks to AGENTS.md
create_symlinks()?;

// Start daemon to watch .rules directory
start_daemon(mpsc::channel().1)?;

// Enable autostart for the daemon
enable_autostart()?;

// Check if autostart is enabled
let enabled = is_autostart_enabled()?;

// Disable autostart
disable_autostart()?;

// Manual single instance lock management (advanced usage)
let _lock = SingleInstanceLock::acquire()?;  // Acquire system-wide lock
// Lock is automatically released when _lock goes out of scope
```

## File Structure

After running `known init` and `known symlink`, your project will have:

```
your-project/
├── AGENTS.md              # Main instruction file
├── CLAUDE.md              # Symlink to AGENTS.md
├── GEMINI.md              # Symlink to AGENTS.md
└── .rules/                # Directory for project-specific rules
    ├── rule1.txt          # Migrated from .cursor/rules/
    └── config.toml        # Migrated from .windsurf/rules/
```

## Rules Directory Migration

Known automatically manages rules directories used by various AI coding assistants:

- **`.cursor/rules`** → **`.rules`**: Files from Cursor's rules directory are moved to the unified `.rules` directory
- **`.windsurf/rules`** → **`.rules`**: Files from Windsurf's rules directory are moved to the unified `.rules` directory

This migration happens automatically when you run `known symlink`. If files with the same name already exist in `.rules`, they will be skipped with a warning message.

## Single Instance Enforcement

The daemon process enforces single instance operation to prevent conflicts and resource contention:

- **Centralized PID File Locking**: Uses a system-wide PID file with exclusive file locking
- **Automatic Cleanup**: PID file is automatically removed when daemon stops gracefully
- **Stale Process Detection**: Detects and handles stale PID files from crashed processes
- **Error Handling**: Provides clear error messages when attempting to start multiple instances

If you try to start a second daemon instance anywhere on the system, you'll see an error message:

```bash
$ known daemon
Error running daemon: Another instance of the daemon is already running
```

The centralized PID file contains the process ID of the running daemon and is automatically cleaned up when the process stops.

## Default AGENTS.md Content

When you run `known init`, if no instruction file exists, it creates an `AGENTS.md` file with default content that provides guidance to agentic coding agents like Claude Code, Gemini CLI, and other AI assistants.

## Development

### Building

```bash
cargo build
```

### Testing

```bash
cargo test
```

### Linting

```bash
cargo clippy
cargo fmt
```

## License

This project is licensed under the BSD 3-Clause License - see the [LICENSE](LICENSE) file for details.
