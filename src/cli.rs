use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "iris")]
#[command(about = "Simple theme switcher", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// List available themes
    List,
    /// Switch to preset (theme)
    Switch { name: String },
    /// Show current status and selected theme
    Status,
}
