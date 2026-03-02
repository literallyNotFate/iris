use clap::Parser;
use iris::cli::{Cli, Commands};

fn main() {
    let cli: Cli = Cli::parse();
    match cli.command {
        Commands::List => handle_list_command(),
        Commands::Switch { name } => handle_switch_command(name),
        Commands::Status => handle_status_command(),
    }
}

/// Handle list available themes command
fn handle_list_command() {
    println!("List command executed!");
}

/// Handle switch command
fn handle_switch_command(theme: String) {
    println!("Swtiched to {}", theme);
}

/// Handle status command
fn handle_status_command() {
    println!("Current theme: ?");
}
