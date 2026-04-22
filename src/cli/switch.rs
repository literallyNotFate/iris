use clap::Args;

#[derive(Args)]
pub struct SwitchArgs {
    /// Name of the theme to apply (e.g., 'melange', 'gruvbox')
    #[arg(value_name = "THEME")]
    pub name: String,

    /// Force fetch palette from Neovim, ignoring cache
    #[arg(short, long)]
    pub force: bool,

    /// Use fallback theme if the requested one is unavailable
    #[arg(short = 'b', long)]
    pub fallback: bool,
}

#[derive(Args)]
pub struct ApplyArgs {
    /// Generator name (e.g., tmux, fzf, alacritty)
    #[arg(value_name = "GENERATOR")]
    pub generator: String,

    /// Override the active theme for this specific application
    #[arg(short, long, value_name = "THEME")]
    pub theme: Option<String>,

    /// Use fallback theme if the requested one is unavailable
    #[arg(short = 'b', long)]
    pub fallback: bool,
}
