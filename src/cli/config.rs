use crate::models::NvimStrategy;
use clap::Subcommand;

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

    /// Set a fallback theme to use when the requested theme is unavailable
    Fallback {
        /// Name of the fallback theme (e.g., 'retrobox')
        #[arg(value_name = "THEME")]
        name: String,
    },
}
