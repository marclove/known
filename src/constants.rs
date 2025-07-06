//! Shared constants used across the application.

/// The directory name for rules files
pub const RULES_DIR: &str = ".rules";

/// The directory name for cursor rules files
pub const CURSOR_RULES_DIR: &str = ".cursor/rules";

/// The directory name for windsurf rules files
pub const WINDSURF_RULES_DIR: &str = ".windsurf/rules";

/// The filename for the agents instruction file (uppercase)
pub const AGENTS_FILENAME: &str = "AGENTS.md";

/// Default content for a new AGENTS.md file
pub const AGENTS_CONTENTS: &str = "# AGENTS\nThis file provides guidance to agentic coding agents like [Claude Code](https://claude.ai/code), [Gemini CLI](https://github.com/google-gemini/gemini-cli), and [Codex CLI](https://github.com/openai/codex) when working with code in this repository.";

/// The filename for the claude instruction file (uppercase)
pub const CLAUDE_FILENAME: &str = "CLAUDE.md";

/// The filename for the gemini instruction file (uppercase)
pub const GEMINI_FILENAME: &str = "GEMINI.md";
