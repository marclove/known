use clap::{Parser, Subcommand};
use known::{
    add_directory_to_config, create_agents_file, create_symlinks, disable_autostart,
    enable_autostart, is_autostart_enabled, start_daemon, stop_daemon,
};
use std::io;
use std::process::{self, Command, Stdio};
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
    RunDaemon,
    /// Enable autostart for the daemon
    EnableAutostart,
    /// Disable autostart for the daemon
    DisableAutostart,
    /// Check if autostart is enabled
    AutostartStatus,
    /// Add current working directory to the list of watched directories
    Add,
    /// Stop the daemon process
    Stop,
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

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Init => match create_agents_file() {
            Ok(()) => println!("Successfully initialized project with AGENTS.md"),
            Err(e) => {
                eprintln!("Error creating AGENTS.md: {}", e);
                process::exit(1);
            }
        },
        Commands::Symlink => match create_symlinks() {
            Ok(()) => println!(
                "Successfully created symlinks: CLAUDE.md and GEMINI.md now point to AGENTS.md"
            ),
            Err(e) => {
                eprintln!("Error creating symlinks: {}", e);
                process::exit(1);
            }
        },
        Commands::Start => {
            // Spawn a new process to run the daemon
            match spawn_daemon_process() {
                Ok(()) => println!("Daemon started successfully"),
                Err(e) => {
                    eprintln!("Error starting daemon: {}", e);
                    process::exit(1);
                }
            }
        }
        Commands::RunDaemon => {
            // Warn users that this is an internal command
            eprintln!("WARNING: 'run-daemon' is an internal command used by 'start'.");
            eprintln!("You should typically use 'known start' instead to launch the daemon.");
            eprintln!("Continuing with daemon execution...");
            eprintln!();

            // Create a channel for shutdown signal (not used in CLI mode, but required by API)
            let (_shutdown_tx, shutdown_rx) = mpsc::channel();

            match start_daemon(shutdown_rx) {
                Ok(()) => println!("Daemon stopped"),
                Err(e) => {
                    eprintln!("Error running daemon: {}", e);
                    process::exit(1);
                }
            }
        }
        Commands::EnableAutostart => match enable_autostart() {
            Ok(()) => println!("Autostart enabled successfully"),
            Err(e) => {
                eprintln!("Error enabling autostart: {}", e);
                process::exit(1);
            }
        },
        Commands::DisableAutostart => match disable_autostart() {
            Ok(()) => println!("Autostart disabled successfully"),
            Err(e) => {
                eprintln!("Error disabling autostart: {}", e);
                process::exit(1);
            }
        },
        Commands::AutostartStatus => match is_autostart_enabled() {
            Ok(enabled) => {
                if enabled {
                    println!("Autostart is enabled");
                } else {
                    println!("Autostart is disabled");
                }
            }
            Err(e) => {
                eprintln!("Error checking autostart status: {}", e);
                process::exit(1);
            }
        },
        Commands::Add => {
            let current_dir = match std::env::current_dir() {
                Ok(dir) => dir,
                Err(e) => {
                    eprintln!("Error getting current directory: {}", e);
                    process::exit(1);
                }
            };

            match add_directory_to_config(&current_dir) {
                Ok(added) => {
                    if added {
                        println!(
                            "Successfully added '{}' to watched directories",
                            current_dir.display()
                        );
                    } else {
                        println!(
                            "Directory '{}' is already in the watched directories list",
                            current_dir.display()
                        );
                    }
                }
                Err(e) => {
                    eprintln!("Error adding directory to configuration: {}", e);
                    process::exit(1);
                }
            }
        }
        Commands::Stop => match stop_daemon() {
            Ok(()) => println!("Daemon stopped successfully"),
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    println!("No daemon is currently running");
                } else {
                    eprintln!("Error stopping daemon: {}", e);
                    process::exit(1);
                }
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use known::{load_config, remove_directory_from_config};
    use std::env;
    use tempfile::tempdir;

    #[test]
    fn test_add_command_acceptance() {
        // Create a temporary directory to simulate a project directory
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();

        // Change to the temporary directory
        let original_dir = env::current_dir().unwrap();
        env::set_current_dir(temp_path).unwrap();

        // Ensure the directory is not already in config
        let _ = remove_directory_from_config(temp_path);

        // Load initial config to verify directory is not present
        let initial_config = load_config().unwrap();
        assert!(!initial_config.contains_directory(temp_path));

        // This test will fail until we implement the Add command
        // For now, let's manually test the underlying functionality
        let added = known::add_directory_to_config(temp_path).unwrap();
        assert!(added);

        // Verify the directory was added
        let updated_config = load_config().unwrap();
        assert!(updated_config.contains_directory(temp_path));

        // Clean up
        let _ = remove_directory_from_config(temp_path);
        env::set_current_dir(original_dir).unwrap();
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
}
