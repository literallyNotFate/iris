use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "iris")]
#[command(about = "CLI theme generator/switcher based on nvim colorcheme", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Suppress detailed output (show only essential updates)
    #[arg(short, long, global = true)]
    pub quiet: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize folders, state and zsh hooks
    Init,

    /// Switch to a theme (defaults to current Neovim theme if name is omitted)
    Switch {
        /// Name of the theme to apply
        name: String,
    },

    /// Sync all apps with current Neovim theme
    Sync,

    /// Show current status, active theme and enabled apps
    Status,

    /// Manage generators through selection
    Gen {
        #[command(subcommand)]
        action: GenAction,
    },
}

#[derive(Subcommand)]
pub enum GenAction {
    /// Automatically enable all installed generators
    Auto,
    /// Enable generator
    Enable { name: String },
    /// Disable generator
    Disable { name: String },
    /// Interactive selection
    Select,
    /// List of all available generators and their status
    List,
}
