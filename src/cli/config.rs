use crate::models::PluginManager;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Show current active configuration
    Show,

    /// Configure Neovim integration
    Nvim {
        /// Force a specific plugin manager (lazy, packer, default)
        #[clap(long, short)]
        manager: Option<PluginManager>,

        /// Run auto-detection and update state
        #[clap(long, short)]
        detect: bool,
    },

    /// Set a fallback theme to use when the requested theme is unavailable
    Fallback {
        /// Name of the fallback theme (e.g., 'retrobox')
        #[arg(value_name = "THEME")]
        name: Option<String>,
    },
}
