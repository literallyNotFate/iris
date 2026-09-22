#[derive(clap::Subcommand)]
pub enum CacheAction {
    /// Clear the generated configurations cache
    #[command(arg_required_else_help = false)]
    Clear {
        /// Target a specific generator's cache
        #[arg(value_name = "GENERATOR")]
        generator: Option<String>,
    },

    /// Nuclear option: purge all cached data and directories
    Purge,

    /// Automatically clean cache from orphaned generators
    Clean,

    /// Remove a specific theme palette from the cache
    Remove {
        /// Name of the theme to delete from cache
        #[arg(value_name = "THEME")]
        theme: String,
    },

    /// List all cached palettes and their sizes
    List,

    /// Show cache directory paths and disk usage
    Info,
}

impl CacheAction {
    pub fn requires_lock(&self) -> bool {
        match self {
            CacheAction::List | CacheAction::Info => false,
            CacheAction::Clear { .. }
            | CacheAction::Purge
            | CacheAction::Clean
            | CacheAction::Remove { .. } => true,
        }
    }
}
