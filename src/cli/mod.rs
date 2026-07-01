use clap::{Parser, Subcommand};

pub mod cache;
pub mod config;
pub mod generator;
pub mod switch;

pub use cache::CacheAction;
pub use config::ConfigAction;
pub use generator::GenAction;

#[derive(Parser)]
#[command(name = "iris")]
#[command(about = "CLI theme generator/switcher based on nvim colorscheme", long_about = None)]
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
    Switch(switch::SwitchArgs),

    /// Sync all applications with the current active theme
    Sync {
        /// Force fetch palette from Neovim, ignoring cache
        #[arg(short, long)]
        force: bool,
    },

    /// Apply theme to a specific generator only
    #[command(arg_required_else_help = true)]
    Apply(switch::ApplyArgs),

    /// Interactive theme selection (with cached ones and installed)
    Select,

    /// Toggle between the current and previous theme seamlessly
    Toggle,

    /// Display current status, active theme, and enabled applications
    Status,

    /// Returns current theme applied in Neovim
    Current,

    /// Display the colors of a theme
    Preview {
        /// Name of the theme to preview (defaults to current)
        #[arg(value_name = "THEME")]
        theme: Option<String>,
    },

    /// Watch for changes in the palette or configuration and re-apply automatically
    Watch {
        /// Debounce interval in milliseconds to prevent flickering during rapid changes
        #[arg(short, long, default_value = "200", value_name = "MS")]
        interval: u64,
    },

    /// Audit system health
    #[command(
        long_about = "Checks for missing config files, broken symlinks, or missing system binaries."
    )]
    Health {
        /// Attempt to automatically fix any detected issues
        #[arg(short, long)]
        fix: bool,
    },

    /// Manage theme generators
    Gen {
        #[command(subcommand)]
        action: GenAction,
    },

    /// Manage application cache
    Cache {
        #[command(subcommand)]
        action: CacheAction,
    },

    /// Global configuration management
    Config {
        #[command(subcommand)]
        action: Option<ConfigAction>,
    },

    /// Internal command for shell completions
    #[command(hide = true)]
    CompleteList,
}
