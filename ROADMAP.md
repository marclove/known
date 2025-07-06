# Roadmap

- [x] Creating the AGENTS.md file is a CLI `init` task rather than the default action when executing the binary.
- [x] If a file named `CLAUDE.md` exists when calling `init`, rename it to `AGENTS.md`.
- [x] If a file named `GEMINI.md` exists when calling `init`, rename it to `AGENTS.md`. If both `CLAUDE.md` and `GEMINI.md` exist, create an empty `AGENTS.md` file and print a detailed and easy to understand instruction to the user that they should manually copy over content from those files into `AGENTS.md`.
- [x] Add a new CLI command `symlink` that symlinks `AGENTS.md` to `CLAUDE.md` and `GEMINI.md`.
- [x] When calling the `init` task, you should also create a new directory (within the current working directory) named `.rules`, if it doesn't already exist. It should gracefully exit if the directory already exists.
- [x] When running the `symlink` command, move any files that exist in the `${cwd}/.cursor/rules` to the `${cwd}.rules` directory. If any of the files already exist, print a user-friendly warning to STDOUT and skip the file.
- [x] When running the `symlink` command, move any files that exist in the `${cwd}/.windsurf/rules` to the `${cwd}.rules` directory. If any of the files already exist, print a user-friendly warning to STDOUT and skip the file.
- [x] Add a CLI command that launches a process that acts as a daemon, watching all the files in `${cwd}.rules` directory and maintaining always-accurate symlinks in the directories `${cwd}/.cursor/rules` and `${cwd}/.windsurf/rules`. This should allow the user to only have to manage rules directory in `${cwd}.rules` and know that the particular IDE rules directories were always kept in sync. The should only require actions when rules files are added or removed from the `${cwd}.rules` directory.
- [x] Add the [auto-launch crate](https://crates.io/crates/auto-launch) and implement cross-platform autostart of the daemon CLI command.
- [x] Enforce that only one instance autolaunched process can be running at any time.
- [x] Centralize the lockfile into the application configuration directory using the `directories` crate to store the file in the right location for each OS platform. This will ensure that only one watch process is running for the entire system.
- [x] Keep a configuration file in the application configuration directory that keeps track of a list of all directories in which the `symlink` task has been executed (these will be code repositories). Then, when the system-wide daemon process is started, it watches all the directories in a single process and keeps their subdirectories synced. This means there will be one system-wide process watching multiple code project directories and keeping their rules files syncronized for all potential code editors that our library supports.
- [x] Add a CLI task named `add` that adds the current working directory to the list of watched directories in our application's configuration file.
- [x] Rename the `daemon` task to `start` and rather run as the daemon itself, it starts the long-running daemon process and exits.
- [x] Add a CLI task called `stop` which gets the PID of the daemon process and makes it gracefully exit. If the daemon is not running, it fails gracefully and writes an easy to understand notification to STDOUT.
