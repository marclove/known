use clap::{Parser, Subcommand};
use known::{
    add_directory_to_config, create_agents_file, create_symlinks, disable_autostart,
    enable_autostart, is_autostart_enabled, start_daemon,
};
use std::process;
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
    Daemon,
    /// Enable autostart for the daemon
    EnableAutostart,
    /// Disable autostart for the daemon
    DisableAutostart,
    /// Check if autostart is enabled
    AutostartStatus,
    /// Add current working directory to the list of watched directories
    Add,
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
        Commands::Daemon => {
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
}
