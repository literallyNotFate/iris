use clap::{Subcommand, ValueEnum};

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Show current active configuration
    Show,

    /// Open config/state file in editor of choice
    Edit,

    /// Set or update a configuration option
    Set {
        /// Configuration key to update
        #[arg(value_enum)]
        key: ConfigKey,

        /// New value for the option (omitting this triggers interactive mode/auto-detect)
        value: Option<String>,
    },

    /// Check health and validity of the configuration and environment
    Check,

    /// Reset configuration file to default factory values
    Reset,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfigKey {
    /// Neovim plugin manager (lazy, packer, default)
    Manager,
    /// Fallback theme name
    Fallback,
}
