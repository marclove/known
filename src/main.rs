use clap::{Parser, Subcommand};
use known::{create_agents_file, create_symlinks, start_daemon};
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
    /// Start daemon to watch .rules directory and maintain symlinks
    Daemon,
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

            match start_daemon(".", shutdown_rx) {
                Ok(()) => println!("Daemon stopped"),
                Err(e) => {
                    eprintln!("Error running daemon: {}", e);
                    process::exit(1);
                }
            }
        }
    }
}
