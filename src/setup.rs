use crate::{
    context::AppContext,
    models::{Palette, State},
    status::Status,
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
        println!("\n{}\n", "Starting Iris Initialization".bold().blue());

        let task = Status::step("Preparing infrastructure...", 0);
        fs::create_dir_all(&ctx.cache_path).context("Failed to create cache directory for Iris")?;
        task.done(Some("Cache directory is ready."));

        let task = Status::step("Initializing application state...", 0);
        Self::setup_initial_state(ctx)?;
        task.done(Some("Application state initialized."));

        let task = Status::step("Integrating with shell...", 0);
        Self::setup_zsh_hook()?;
        task.done(Some("Shell integration complete."));

        println!("\n{}", "Setup complete! Ready to sync.".green().bold());
        Ok(())
    }

    fn setup_initial_state(ctx: &AppContext) -> Result<()> {
        let path: PathBuf = ctx.base_path.join("state.json");
        if path.exists() {
            Status::success("Found existing state.json, skipping.", 1);
            return Ok(());
        }

        let task = Status::step("Detecting active Neovim theme...", 1);
        let current_theme = Palette::current()
            .context("Neovim theme not detected. Please set a colorscheme in Neovim first.")?;
        task.done(Some(&format!(
            "Detected theme: {}",
            current_theme.bold().cyan()
        )));

        let scan_task = Status::step("Scanning for installed tools...", 1);
        let mut enabled = Vec::new();
        let home: PathBuf = dirs::home_dir().context("Could not find home directory")?;

        if home.join(".config/ghostty").exists() && which::which("ghostty").is_ok() {
            enabled.push("ghostty".to_string());
            Status::success("Ghostty found", 2);
        }

        if home.join(".zshrc").exists() || home.join(".config/zsh").exists() {
            enabled.push("fzf".to_string());
            Status::success("fzf found", 2);
        }

        if which::which("bat").is_ok() {
            enabled.push("bat".to_string());
            Status::success("bat found", 2);
        }

        let initial_state: State = State::new(current_theme, enabled);
        initial_state.save_to(&path)?;

        scan_task.done(Some("Scanning complete and state.json created."));
        Ok(())
    }

    pub fn setup_zsh_hook() -> Result<()> {
        let home = dirs::home_dir().context("Home dir not found")?;
        let zshrc = home.join(".zshrc");

        if !zshrc.exists() {
            Status::error(".zshrc not found, skipping hook injection.", 1);
            return Ok(());
        }

        let hook_id: &str = "# --- Iris FZF Sync ---";
        let content: String = fs::read_to_string(&zshrc)?;

        if content.contains(hook_id) {
            Status::success("Zsh hook already present in .zshrc.", 1);
            return Ok(());
        }

        let cache_file_path = home.join(".cache/iris/fzf.sh");
        let task = Status::step("Injecting Zsh hook...", 1);

        let hook: String = format!(
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

        let full_hook: String = format!("\n{}\n", hook);

        let mut file = OpenOptions::new().append(true).open(&zshrc)?;
        writeln!(file, "{}", full_hook)?;

        task.done(Some("Zsh hook successfully injected into .zshrc."));
        Ok(())
    }
}
