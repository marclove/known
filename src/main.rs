use clap::{Parser, Subcommand};
use known::create_agents_file;
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
    }
}
