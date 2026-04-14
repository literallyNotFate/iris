use crate::modules::GeneratorType;
use clap::{Parser, Subcommand, ValueEnum};

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

    /// Apply theme to a specific generator
    Apply {
        /// Name of the generator (tmux, starship, btop, etc.)
        generator: String,

        /// Optional: specific theme name
        #[arg(short, long)]
        theme: Option<String>,
    },

    /// Show current status, active theme and enabled apps
    Status,

    /// Watch for changes in palette/config and re-apply automatically
    Watch {
        /// Custom debounce interval in milliseconds
        #[arg(short, long, default_value = "200")]
        interval: u64,
    },

    /// Check whether there are any problems with application before applying
    Health {
        /// Automatically fix detected issues
        #[arg(short, long)]
        fix: bool,
    },

    /// Manage generators through selection
    Gen {
        #[command(subcommand)]
        action: GenAction,
    },
}

#[derive(ValueEnum, Clone, Copy, PartialEq, Eq)]
pub enum StatusFilter {
    /// Enabled and installed
    Active,
    /// Installed but disabled
    Ready,
    /// Enabled but program not found in system
    Broken,
    /// Disabled and not found
    Missing,
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
    List {
        /// Filter by type (terminal, tool, etc.)
        #[arg(short = 't', long = "type")]
        generator_type: Option<GeneratorType>,

        /// Filter by state (active, ready, broken, missing)
        #[arg(short = 's', long = "status")]
        status: Option<StatusFilter>,
    },
}
