use clap::{Parser, Subcommand};
use known::{create_agents_file, create_symlinks};
use std::process;

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
    }
}
