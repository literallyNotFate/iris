use crate::{models::NvimStrategy, modules::GeneratorType};
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
    /// Initialize folders, state, and shell hooks
    #[command(
        long_about = "Sets up the necessary directory structure (~/.cache/iris, etc.) and prepares shell integration hooks."
    )]
    Init,

    /// Switch to a specific theme
    #[command(arg_required_else_help = true)]
    Switch {
        /// Name of the theme to apply (e.g., 'melange', 'gruvbox')
        #[arg(value_name = "THEME")]
        name: String,

        /// Force fetch palette from Neovim, ignoring cache
        #[arg(short, long)]
        force: bool,
    },

    /// Sync all applications with the current active theme
    Sync {
        /// Force fetch palette from Neovim, ignoring cache
        #[arg(short, long)]
        force: bool,
    },

    /// Apply theme to a specific generator only
    #[command(arg_required_else_help = true)]
    Apply {
        /// Generator name (e.g., tmux, fzf, alacritty)
        #[arg(value_name = "GENERATOR")]
        generator: String,

        /// Override the active theme for this specific application
        #[arg(short, long, value_name = "THEME")]
        theme: Option<String>,
    },

    /// Display current status, active theme, and enabled applications
    Status,

    /// Watch for changes in the palette or configuration and re-apply automatically
    Watch {
        /// Debounce interval in milliseconds to prevent flickering during rapid changes
        #[arg(short, long, default_value = "200", value_name = "MS")]
        interval: u64,
    },

    /// Audit system health and application configurations
    #[command(
        long_about = "Checks for missing config files, broken symlinks, or missing system binaries."
    )]
    Health {
        /// Attempt to automatically fix any detected issues
        #[arg(short, long)]
        fix: bool,
    },

    /// Manage theme generators (enable/disable apps)
    Gen {
        #[command(subcommand)]
        action: GenAction,
    },

    /// Manage application cache and generated files
    Cache {
        #[command(subcommand)]
        action: CacheAction,
    },

    /// Global configuration management
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand)]
pub enum GenAction {
    /// Automatically enable generators for all supported apps found in the system
    Auto,

    /// Enable a specific generator
    Enable {
        #[arg(value_name = "GENERATOR")]
        name: String,
    },

    /// Disable a specific generator
    Disable {
        #[arg(value_name = "GENERATOR")]
        name: String,
    },

    /// Open an interactive TUI selector to manage generators
    Select,

    /// List available generators and their current status
    List {
        /// Filter generators by type (e.g., 'tool', 'terminal')
        #[arg(short = 't', long = "type", value_name = "TYPE")]
        generator_type: Option<GeneratorType>,

        /// Filter generators by their operational state
        #[arg(short = 's', long = "status", value_name = "STATE")]
        status: Option<StatusFilter>,
    },
}

#[derive(Subcommand)]
pub enum CacheAction {
    /// Clear the generated configurations cache
    #[command(arg_required_else_help = false)]
    Clear {
        /// Target a specific generator's cache
        #[arg(value_name = "GENERATOR")]
        generator: Option<String>,

        /// Nuclear option: clear everything (history, downloaded themes, and configs)
        #[arg(short, long)]
        all: bool,
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
pub enum ConfigAction {
    /// Configure Neovim integration
    Nvim {
        /// Force a specific strategy (lazy, packer, default)
        #[clap(long, short)]
        strategy: Option<NvimStrategy>,

        /// Run auto-detection and update state
        #[clap(long, short)]
        detect: bool,
    },
}
