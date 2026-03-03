use crate::context::AppContext;
use crate::models::State;
use anyhow::{Context as _, Result};
use colored::Colorize;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

pub struct Setup;

impl Setup {
    /// Main initializing function
    pub fn run(ctx: &AppContext) -> Result<()> {
        Self::setup_initial_state(ctx)?;
        Self::setup_zsh_hook()?;
        Ok(())
    }

    /// Setup initial state with enabled clis
    fn setup_initial_state(ctx: &AppContext) -> Result<()> {
        let path: PathBuf = ctx.base_path.join("state.json");
        if path.exists() {
            return Ok(());
        }

        let mut enabled = Vec::new();
        let home: PathBuf = dirs::home_dir().context("Could not find home directory")?;

        if home.join(".config/ghostty").exists() {
            enabled.push("ghostty".to_string());
        }
        if home.join(".zshrc").exists() || home.join(".config/zsh").exists() {
            enabled.push("fzf".to_string());
        }

        let initial_state = State {
            current_theme: "melange".to_string(),
            enabled_generators: enabled,
        };

        initial_state.save_to(&path)?;
        println!("  {} Created initial state.json", "✔".green());
        Ok(())
    }

    /// Install zsh hook to catch file change
    pub fn setup_zsh_hook() -> Result<()> {
        let zshrc = dirs::home_dir()
            .context("Home dir not found")?
            .join(".zshrc");

        if !zshrc.exists() {
            return Ok(());
        }

        let hook_id = "# --- Iris FZF Sync ---";
        let content = fs::read_to_string(&zshrc)?;

        if content.contains(hook_id) {
            println!("  {} Zsh hook already exists, skipping", "ℹ".blue());
            return Ok(());
        }

        let hook = r#"
        # --- Iris FZF Sync ---
        autoload -Uz add-zsh-hook
        _iris_fzf_sync() {
            local cache_file="$HOME/.cache/iris/fzf.sh"
            if [[ -f "$cache_file" ]]; then
                local mt=$(stat -f %m "$cache_file" 2>/dev/null || stat -c %Y "$cache_file" 2>/dev/null)
                if [[ "$mt" != "$LAST_IRIS_SYNC" ]]; then
                    source "$cache_file"
                    export LAST_IRIS_SYNC="$mt"
                fi
            fi
        }
        add-zsh-hook precmd _iris_fzf_sync
        # ---------------------
        "#;

        let mut file = OpenOptions::new().append(true).open(&zshrc)?;
        writeln!(file, "{}", hook)?;

        println!("  {} Injected Zsh hook into .zshrc", "✔".green());
        Ok(())
    }
}
