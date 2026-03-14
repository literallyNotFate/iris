use crate::{
    context::AppContext,
    models::{Palette, State},
};
use anyhow::{Context as _, Result};
use colored::Colorize;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
};

pub struct Setup;

impl Setup {
    pub fn run(ctx: &AppContext) -> Result<()> {
        fs::create_dir_all(&ctx.cache_path)?;

        Self::setup_initial_state(ctx)?;
        Self::setup_zsh_hook()?;
        Ok(())
    }

    fn setup_initial_state(ctx: &AppContext) -> Result<()> {
        let path: PathBuf = ctx.base_path.join("state.json");
        if path.exists() {
            return Ok(());
        }

        let current_theme = Palette::current()
            .context("Failed to initialize: Neovim theme not detected. Please set a colorscheme in Neovim first.")?;

        let mut enabled = Vec::new();
        let home: PathBuf = dirs::home_dir().context("Could not find home directory")?;

        if home.join(".config/ghostty").exists() && which::which("ghostty").is_ok() {
            enabled.push("ghostty".to_string());
        }

        if home.join(".zshrc").exists() || home.join(".config/zsh").exists() {
            enabled.push("fzf".to_string());
        }

        if which::which("bat").is_ok() {
            enabled.push("bat".to_string());
        }

        let initial_state = State {
            current_theme,
            enabled_generators: enabled,
        };

        initial_state.save_to(&path)?;
        println!("  {} Created initial state.json", "✔".green());
        Ok(())
    }

    pub fn setup_zsh_hook() -> Result<()> {
        let home = dirs::home_dir().context("Home dir not found")?;
        let zshrc = home.join(".zshrc");

        if !zshrc.exists() {
            return Ok(());
        }

        let hook_id = "# --- Iris FZF Sync ---";
        let content = fs::read_to_string(&zshrc)?;

        if content.contains(hook_id) {
            println!("  {} Zsh hook already exists, skipping", "ℹ".blue());
            return Ok(());
        }

        let cache_file_path = home.join(".cache/iris/fzf.sh");

        let hook = format!(
            r#"
# --- Iris FZF Sync ---
autoload -Uz add-zsh-hook
_iris_fzf_sync() {{
    local cache_file="{}"
    if [[ -f "$cache_file" ]]; then
        local mt=$(stat -f %m "$cache_file" 2>/dev/null || stat -c %Y "$cache_file" 2>/dev/null)
        if [[ "$mt" != "$LAST_IRIS_SYNC" ]]; then
            source "$cache_file"
            export LAST_IRIS_SYNC="$mt"
        fi
    fi
}}
add-zsh-hook precmd _iris_fzf_sync
# ---------------------
"#,
            cache_file_path.display()
        );

        let mut file = OpenOptions::new().append(true).open(&zshrc)?;
        writeln!(file, "{}", hook)?;

        println!("  {} Injected Zsh hook into .zshrc", "✔".green());
        Ok(())
    }
}
