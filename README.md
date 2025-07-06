# Known

A Rust library and CLI tool for managing agentic LLM instruction files in your projects.

## Overview

Known helps you create and manage instruction files for various AI coding assistants like Claude Code, Gemini CLI, and other agentic tools. It provides a unified `AGENTS.md` file format with automatic symlink generation for compatibility with different naming conventions.

## Features

- **Unified instruction file**: Creates `AGENTS.md` as the single source of truth
- **Automatic migration**: Renames existing `CLAUDE.md` or `GEMINI.md` files to `AGENTS.md`
- **Symlink generation**: Creates `CLAUDE.md` and `GEMINI.md` symlinks pointing to `AGENTS.md`
- **Rules directory management**: Automatically creates `.rules` directory and migrates files from `.cursor/rules` and `.windsurf/rules`
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

## Library Usage

You can also use Known as a Rust library:

```rust
use known::{create_agents_file, create_symlinks};

// Create AGENTS.md file
create_agents_file()?;

// Create symlinks to AGENTS.md
create_symlinks()?;
```

## File Structure

After running `known init` and `known symlink`, your project will have:

```
your-project/
├── AGENTS.md          # Main instruction file
├── CLAUDE.md          # Symlink to AGENTS.md
├── GEMINI.md          # Symlink to AGENTS.md
└── .rules/            # Directory for project-specific rules
    ├── rule1.txt      # Migrated from .cursor/rules/
    └── config.toml    # Migrated from .windsurf/rules/
```

## Rules Directory Migration

Known automatically manages rules directories used by various AI coding assistants:

- **`.cursor/rules`** → **`.rules`**: Files from Cursor's rules directory are moved to the unified `.rules` directory
- **`.windsurf/rules`** → **`.rules`**: Files from Windsurf's rules directory are moved to the unified `.rules` directory

This migration happens automatically when you run `known symlink`. If files with the same name already exist in `.rules`, they will be skipped with a warning message.

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
