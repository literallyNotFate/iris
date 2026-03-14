use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "iris")]
#[command(about = "CLI theme generator/switcher based on nvim colorcheme", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize folders, state and zsh hooks
    Init,

    /// Switch to a theme (defaults to current Neovim theme if name is omitted)
    Switch {
        /// Name of the theme to apply
        name: Option<String>,
    },

    /// Show current status, active theme and enabled apps
    Status,
}
