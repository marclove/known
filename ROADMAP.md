# Roadmap

- [x] Creating the AGENTS.md file is a CLI `init` task rather than the default action when executing the binary.
- [x] If a file named `CLAUDE.md` exists when calling `init`, rename it to `AGENTS.md`.
- [x] If a file named `GEMINI.md` exists when calling `init`, rename it to `AGENTS.md`. If both `CLAUDE.md` and `GEMINI.md` exist, create an empty `AGENTS.md` file and print a detailed and easy to understand instruction to the user that they should manually copy over content from those files into `AGENTS.md`.
- [x] Add a new CLI command `symlink` that symlinks `AGENTS.md` to `CLAUDE.md` and `GEMINI.md`.
- [x] When calling the `init` task, you should also create a new directory (within the current working directory) named `.rules`, if it doesn't already exist. It should gracefully exit if the directory already exists.
- [x] When running the `symlink` command, move any files that exist in the `${cwd}/.cursor/rules` to the `${cwd}.rules` directory. If any of the files already exist, print a user-friendly warning to STDOUT and skip the file.
- [x] When running the `symlink` command, move any files that exist in the `${cwd}/.windsurf/rules` to the `${cwd}.rules` directory. If any of the files already exist, print a user-friendly warning to STDOUT and skip the file.
