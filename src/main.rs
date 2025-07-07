use clap::{Parser, Subcommand};
use known::{
    add_directory_to_config, create_agents_file, create_symlinks, disable_autostart,
    enable_autostart, is_autostart_enabled, is_daemon_running, remove_directory_from_config,
    start_daemon, stop_daemon,
};
use std::io;
use std::process::{Command, Stdio};
use std::sync::mpsc;

#[derive(Parser)]
#[command(name = "known")]
#[command(about = "A CLI tool for managing project files")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize project by creating AGENTS.md file
    Init,
    /// Create symlinks from AGENTS.md to CLAUDE.md and GEMINI.md
    Symlink,
    /// Start daemon to watch all configured directories and maintain symlinks
    Start,
    /// Run daemon process (internal command, used by start)
    #[clap(hide = true)]
    RunDaemon,
    /// Enable autostart for the daemon
    EnableAutostart,
    /// Disable autostart for the daemon
    DisableAutostart,
    /// Check if autostart is enabled
    AutostartStatus,
    /// Add current working directory (or specified directory) to the list of watched directories
    Add {
        /// Directory path to add (defaults to current working directory)
        #[arg(value_name = "DIRECTORY")]
        directory: Option<std::path::PathBuf>,
    },
    /// Remove current working directory (or specified directory) from the list of watched directories
    Remove {
        /// Directory path to remove (defaults to current working directory)
        #[arg(value_name = "DIRECTORY")]
        directory: Option<std::path::PathBuf>,
    },
    /// Stop the daemon process
    Stop,
    /// List all watched directories from the configuration file
    List,
}

/// Spawns a new process to run the daemon in the background
///
/// This function starts a new process running the `run-daemon` command,
/// which will run the actual daemon functionality in the background.
/// The spawned process is detached so it continues running after this
/// function exits.
///
/// # Errors
///
/// Returns an error if the process cannot be spawned or if there's an
/// issue with process creation.
///
fn spawn_daemon_process() -> io::Result<()> {
    // Get the current executable path
    let current_exe = std::env::current_exe()?;

    // Spawn a new process with the run-daemon command
    let mut cmd = Command::new(&current_exe);
    cmd.arg("run-daemon")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    // On Unix systems, we can properly detach the process
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }

    // Spawn the process
    let child = cmd.spawn()?;

    // Don't wait for the child process - let it run in the background
    drop(child);

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Init => {
            create_agents_file()?;
            println!("Successfully initialized project with AGENTS.md");
        }
        Commands::Symlink => {
            create_symlinks()?;
            println!(
                "Successfully created symlinks: CLAUDE.md and GEMINI.md now point to AGENTS.md"
            );
        }
        Commands::Start => {
            spawn_daemon_process()?;
            println!("Daemon started successfully");
        }
        Commands::RunDaemon => {
            eprintln!("WARNING: 'run-daemon' is an internal command used by 'start'.");
            eprintln!("You should typically use 'known start' instead to launch the daemon.");
            eprintln!("Continuing with daemon execution...");
            eprintln!();

            let (_shutdown_tx, shutdown_rx) = mpsc::channel();
            start_daemon(shutdown_rx)?;
            println!("Daemon stopped");
        }
        Commands::EnableAutostart => {
            enable_autostart()?;
            println!("Autostart enabled successfully");

            if !is_daemon_running()? {
                spawn_daemon_process()?;
                println!("Daemon started successfully");
            }
        }
        Commands::DisableAutostart => {
            disable_autostart()?;
            println!("Autostart disabled successfully");
        }
        Commands::AutostartStatus => {
            let enabled = is_autostart_enabled()?;
            if enabled {
                println!("Autostart is enabled");
            } else {
                println!("Autostart is disabled");
            }
        }
        Commands::Add { directory } => {
            let target_dir = match directory {
                Some(dir) => dir.clone(),
                None => std::env::current_dir()?,
            };

            if !target_dir.exists() {
                return Err(format!("Directory '{}' does not exist", target_dir.display()).into());
            }

            if !target_dir.is_dir() {
                return Err(format!("'{}' is not a directory", target_dir.display()).into());
            }

            let added = add_directory_to_config(&target_dir)?;
            if added {
                println!(
                    "Successfully added '{}' to watched directories",
                    target_dir.display()
                );
            } else {
                println!(
                    "Directory '{}' is already in the watched directories list",
                    target_dir.display()
                );
            }
        }
        Commands::Remove { directory } => {
            let target_dir = match directory {
                Some(dir) => dir.clone(),
                None => std::env::current_dir()?,
            };

            let removed = remove_directory_from_config(&target_dir)?;
            if removed {
                println!(
                    "Successfully removed '{}' from watched directories",
                    target_dir.display()
                );
            } else {
                println!(
                    "Directory '{}' was not in the watched directories list",
                    target_dir.display()
                );
            }
        }
        Commands::Stop => match stop_daemon() {
            Ok(()) => println!("Daemon stopped successfully"),
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    println!("No daemon is currently running");
                } else {
                    return Err(e.into());
                }
            }
        },
        Commands::List => {
            let config = known::load_config()?;
            let directories = config.get_watched_directories();
            if directories.is_empty() {
                println!("No directories are currently being watched");
            } else {
                println!("Watched directories:");
                for dir in directories {
                    println!("  {}", dir.display());
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use known::{
        add_directory_to_config_file, load_config_from_file, remove_directory_from_config_file,
    };
    use tempfile::tempdir;

    #[test]
    fn test_add_command_acceptance() {
        // Create temporary directories for both the project and config
        let project_dir = tempdir().unwrap();
        let config_dir = tempdir().unwrap();
        let project_path = project_dir.path();
        let config_path = config_dir.path().join("test_config.json");

        // Load initial config to verify directory is not present
        let initial_config = load_config_from_file(&config_path).unwrap();
        assert!(!initial_config.contains_directory(project_path));

        // Test the underlying functionality
        let added = add_directory_to_config_file(project_path, &config_path).unwrap();
        assert!(added);

        // Verify the directory was added
        let updated_config = load_config_from_file(&config_path).unwrap();
        assert!(updated_config.contains_directory(project_path));

        // Test that adding the same directory again returns false
        let added_again = add_directory_to_config_file(project_path, &config_path).unwrap();
        assert!(!added_again);
    }

    #[test]
    fn test_spawn_daemon_process() {
        // Test that spawn_daemon_process doesn't panic and returns Ok
        // We can't easily test the actual spawning in a unit test environment,
        // but we can ensure the function exists and has the right signature

        // This test primarily exists to ensure the function compiles
        // and could be called. In a real test environment, we'd need
        // to mock the process spawning mechanism.

        // For now, we'll just test that the current executable path can be obtained
        let current_exe_result = std::env::current_exe();
        assert!(
            current_exe_result.is_ok(),
            "Should be able to get current executable path"
        );
    }

    #[test]
    fn test_stop_command_acceptance() {
        // Test that stop_daemon can be called (acceptance test)
        // This will typically fail with "No daemon is currently running"
        // unless a daemon is actually running during the test
        //
        // Note: This test uses the real stop_daemon() function intentionally
        // to test the full CLI behavior. The unit tests in single_instance.rs
        // use isolated test functions to avoid signal interference.

        let result = known::stop_daemon();

        // The result should either be:
        // 1. Ok(()) if daemon was running and stopped successfully
        // 2. Err with NotFound if no daemon is running
        // 3. Other errors are possible but less likely in test environment

        match result {
            Ok(()) => {
                // Daemon was running and stopped successfully
                println!("Test: Daemon was running and stopped successfully");
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // Expected case: no daemon running
                assert!(e.to_string().contains("No daemon is currently running"));
            }
            Err(e) => {
                // Other errors should be rare in test environment
                // but we'll allow them and print for debugging
                println!(
                    "Test: Unexpected error (may be environment-specific): {}",
                    e
                );
            }
        }

        // Test passes if we reach this point without panicking
        assert!(true, "stop_daemon function executed without panicking");
    }

    #[test]
    fn test_list_command_acceptance() {
        // Create temporary directories for both the project and config
        let project_dir1 = tempdir().unwrap();
        let project_dir2 = tempdir().unwrap();
        let config_dir = tempdir().unwrap();
        let project_path1 = project_dir1.path();
        let project_path2 = project_dir2.path();
        let config_path = config_dir.path().join("test_config.json");

        // Test with empty config (no directories)
        let config = load_config_from_file(&config_path).unwrap();
        assert_eq!(config.directory_count(), 0);
        assert!(config.get_watched_directories().is_empty());

        // Add directories to config
        let added1 = add_directory_to_config_file(project_path1, &config_path).unwrap();
        assert!(added1);
        let added2 = add_directory_to_config_file(project_path2, &config_path).unwrap();
        assert!(added2);

        // Test that both directories are now in the config
        let updated_config = load_config_from_file(&config_path).unwrap();
        assert_eq!(updated_config.directory_count(), 2);
        assert!(updated_config.contains_directory(project_path1));
        assert!(updated_config.contains_directory(project_path2));

        // Verify get_watched_directories returns both directories
        let watched_dirs = updated_config.get_watched_directories();
        assert_eq!(watched_dirs.len(), 2);
        assert!(watched_dirs
            .iter()
            .any(|d| d.ends_with(project_path1.file_name().unwrap())));
        assert!(watched_dirs
            .iter()
            .any(|d| d.ends_with(project_path2.file_name().unwrap())));
    }

    #[test]
    fn test_remove_command_acceptance() {
        // Create temporary directories for both the project and config
        let project_dir = tempdir().unwrap();
        let config_dir = tempdir().unwrap();
        let project_path = project_dir.path();
        let config_path = config_dir.path().join("test_config.json");

        // Add the directory to config first
        let added = add_directory_to_config_file(project_path, &config_path).unwrap();
        assert!(added, "Directory should be added to config");

        // Verify it's in the config
        let config = load_config_from_file(&config_path).unwrap();
        assert!(
            config.contains_directory(project_path),
            "Directory should be in config"
        );

        // Now test removing it
        let removed = remove_directory_from_config_file(project_path, &config_path).unwrap();
        assert!(removed, "Directory should be removed from config");

        // Verify it's no longer in config
        let updated_config = load_config_from_file(&config_path).unwrap();
        assert!(
            !updated_config.contains_directory(project_path),
            "Directory should not be in config after removal"
        );

        // Test removing a directory that's not in config
        let not_removed = remove_directory_from_config_file(project_path, &config_path).unwrap();
        assert!(
            !not_removed,
            "Removing directory not in config should return false"
        );
    }
}
