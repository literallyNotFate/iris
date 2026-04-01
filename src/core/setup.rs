use crate::{
    core::IrisContext,
    models::{Palette, State},
    utils::{Status, Task},
};
use anyhow::{Context as _, Result};
use colored::Colorize;
use std::{
    collections::BTreeSet,
    fs::{self, OpenOptions},
    io::Write,
};

/// Struct for initializing state of application
pub struct IrisSetup;

impl IrisSetup {
    pub fn run(ctx: &IrisContext) -> Result<()> {
        println!("\n{}\n", "Starting Iris Initialization".bold().blue());

        let task = Status::step("Preparing infrastructure...", 0);
        ctx.paths.ensure_dirs()?;
        task.info(&format!("Data directory: {}", ctx.paths.config.display()));
        task.done(Some("File system is ready."));

        let task = Status::step("Initializing application state...", 0);
        Self::setup_initial_state(ctx, &task)?;
        task.done(Some("Application state initialized."));

        let task = Status::step("Integrating with shell...", 0);
        Self::setup_zsh_hook(ctx, &task)?;
        task.done(Some("Shell integration complete."));

        println!("\n{}", "Setup complete! Ready to sync.".green().bold());
        Ok(())
    }

    fn setup_initial_state(ctx: &IrisContext, parent_task: &Task) -> Result<()> {
        if ctx.paths.state_file.exists() {
            parent_task.info("Found existing state.json, loading current configuration.");
            return Ok(());
        }

        parent_task.info("Detecting active Neovim theme...");
        let current_theme = Palette::current()
            .context("Neovim theme not detected. Please set a colorscheme in Neovim first.")?;

        parent_task.info("Scanning for compatible tools...");
        let generators = crate::modules::all_generators();

        let enabled: Vec<String> = generators
            .iter()
            .filter_map(|g| {
                if g.is_installed() {
                    parent_task.info(&format!("Found {}", g.name().cyan()));
                    Some(g.name().to_string())
                } else {
                    None
                }
            })
            .collect();

        if enabled.is_empty() {
            parent_task.warn(
                "No compatible tools found. You can enable them later via 'iris gen select'.",
            );
        }

        let enabled_set: BTreeSet<String> = enabled.into_iter().collect();
        let initial_state: State = State::new(current_theme, enabled_set);
        initial_state.save_to(&ctx.paths.state_file)?;

        parent_task.info(&format!(
            "State saved to {}",
            ctx.paths.state_file.display()
        ));
        Ok(())
    }

    pub fn setup_zsh_hook(ctx: &IrisContext, parent_task: &Task) -> Result<()> {
        let home = dirs::home_dir().context("Home dir not found")?;
        let zshrc = home.join(".zshrc");

        if !zshrc.exists() {
            Status::warn(".zshrc not found, skipping hook injection.", 1);
            return Ok(());
        }

        let hook_id: &str = "# --- Iris FZF Sync ---";
        let content: String = fs::read_to_string(&zshrc)?;

        if content.contains(hook_id) {
            parent_task.info("Zsh hook already present in .zshrc.");
            return Ok(());
        }

        let cache_file = ctx.paths.cache.join("fzf.sh");
        parent_task.info("Injecting synchronization hook into .zshrc...");

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
            cache_file.display()
        );

        let full_hook: String = format!("\n{}\n", hook);

        let mut file = OpenOptions::new().append(true).open(&zshrc)?;
        writeln!(file, "{}", full_hook)?;

        parent_task.info("Hook successfully appended.");
        Ok(())
    }
}
