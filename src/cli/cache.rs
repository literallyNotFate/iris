use clap::Subcommand;

#[derive(Subcommand)]
pub enum CacheAction {
    /// Clear the generated configurations cache
    #[command(arg_required_else_help = false)]
    Clear {
        /// Target a specific generator's cache
        #[arg(value_name = "GENERATOR")]
        generator: Option<String>,

        /// Nuclear option: clear everything
        #[arg(short, long)]
        all: bool,
    },

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
