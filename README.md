# Known

A Rust library and CLI tool for managing agentic LLM instruction files in your projects.

## Overview

Known helps you create and manage instruction files for various AI coding assistants like Claude Code, Gemini CLI, and other agentic tools. It provides a unified `AGENTS.md` file format with automatic symlink generation for compatibility with different naming conventions.

## Features

- **Unified instruction file**: Creates `AGENTS.md` as the single source of truth
- **Automatic migration**: Renames existing `CLAUDE.md` or `GEMINI.md` files to `AGENTS.md`
- **Symlink generation**: Creates `CLAUDE.md` and `GEMINI.md` symlinks pointing to `AGENTS.md`
- **Cross-platform compatibility**: Works on Unix and Windows systems
- **CLI interface**: Simple command-line tool for project initialization

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

### Create symlinks

Generate compatibility symlinks for different AI tools:

```bash
known link
```

This creates:
- `CLAUDE.md` → `AGENTS.md` (symlink)
- `GEMINI.md` → `AGENTS.md` (symlink)

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

```
your-project/
├── AGENTS.md          # Main instruction file
├── CLAUDE.md          # Symlink to AGENTS.md
└── GEMINI.md          # Symlink to AGENTS.md
```

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
