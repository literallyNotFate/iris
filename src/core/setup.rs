use crate::{
    core::{IrisContext, NeovimBridge, ThemeOrchestrator},
    log::Activity,
    models::PluginManager,
    utils,
};
use anyhow::{Context as _, Result};
use colored::Colorize;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
};

/// Struct for initializing state of application
pub struct IrisSetup;

impl IrisSetup {
    pub fn run(ctx: &mut IrisContext) -> Result<()> {
        if ctx.log.is_detailed() {
            println!();
            println!("{}  {}", "󰒓".purple().bold(), "Iris initialization".bold());
            println!();
        }

        ctx.log
            .action("Infrastructure prepared", || ctx.paths.ensure_dirs())?;
        println!();

        {
            let task =
                ctx.log
                    .step_with_icon(&"󰏘".red().bold(), "Initializing application state", false);
            Self::setup_initial_state(ctx, &task)?;
            task.done_with("System state initialized");
        }

        {
            let task =
                ctx.log
                    .step_with_icon(&"󰒍".green().bold(), "Integrating with shell (zsh)", true);
            Self::setup_zsh_hook(ctx, &task)?;
            task.done_with("`zsh` synchronization hook installed");
        }

        ctx.log
            .success("Iris is now fully configured and ready to go!");
        println!();

        Ok(())
    }

    fn setup_initial_state(ctx: &mut IrisContext, task: &Activity) -> Result<()> {
        if ctx.paths.state_file.exists() {
            task.info("Found existing state.json, loading configuration.");
            return Ok(());
        }

        task.info("Detecting Neovim plugin manager...");
        let manager: PluginManager = NeovimBridge::detect_manager(&ctx.paths);

        if manager != PluginManager::Default {
            let count: usize = NeovimBridge::count_plugins(&ctx.paths, &manager);
            task.info(&format!(
                "Found {} with {} plugins installed.",
                manager,
                count.to_string().yellow().bold()
            ));
        }

        ctx.state.manager = manager;
        task.info("Detecting active Neovim theme...");

        let orchestrator = ThemeOrchestrator::new(&ctx.paths, &task.log);
        let current_theme = orchestrator
            .get_current_theme()
            .unwrap_or_else(|_| "".to_string());

        task.info("Scanning for compatible tools...");
        let installed = ctx.registry.installed();

        if installed.is_empty() {
            task.info("No compatible tools found.");
        } else {
            for generator in installed.iter() {
                task.info(&format!("Found: {}", generator.name().green().bold()));
                ctx.state.enable_generator(generator.name());
            }
        }

        ctx.state.set_theme(current_theme);
        ctx.save()?;

        task.info(&format!(
            "Configuration persisted to {}",
            utils::pretty_path(&ctx.paths.state_file).dimmed()
        ));
        Ok(())
    }

    pub fn setup_zsh_hook(ctx: &IrisContext, task: &Activity) -> anyhow::Result<()> {
        let home: PathBuf = dirs::home_dir().context("Home directory not found")?;
        let zshrc: PathBuf = home.join(".zshrc");

        if !zshrc.exists() {
            ctx.log.warn(".zshrc not found, skipping hook injection.");
            return Ok(());
        }

        let hook_id: &str = "# --- Iris FZF Sync ---";
        let content: String = fs::read_to_string(&zshrc)
            .with_context(|| format!("Failed to read {}", utils::pretty_path(&zshrc)))?;

        if content.contains(hook_id) {
            task.info("`zsh` hook already present in .zshrc.");
            return Ok(());
        }

        let cache_file: PathBuf = ctx.paths.cache.join("fzf.sh");
        task.info("Injecting synchronization hook into .zshrc...");

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

        let mut file = OpenOptions::new()
            .append(true)
            .open(&zshrc)
            .with_context(|| {
                format!(
                    "Failed to open {} for appending",
                    utils::pretty_path(&zshrc)
                )
            })?;

        writeln!(file, "\n{}", hook.trim())
            .with_context(|| format!("Failed to write to {}", utils::pretty_path(&zshrc)))?;

        task.info("Hook successfully appended.");
        Ok(())
    }
}

/// Unit-tests for setup
#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::tests::mock_context;
    use temp_env;

    #[test]
    fn should_handle_setup_zsh_hook_injection() {
        let (tmp, ctx) = mock_context();
        let fake_home = tmp.path();
        let zshrc_path = fake_home.join(".zshrc");

        fs::write(&zshrc_path, "export PATH=$HOME/bin:$PATH\n").unwrap();

        temp_env::with_var("HOME", Some(fake_home), || {
            let task = ctx.log.step("Test Task", true);

            let result = IrisSetup::setup_zsh_hook(&ctx, &task);
            assert!(result.is_ok());

            let updated_content = fs::read_to_string(&zshrc_path).unwrap();

            assert!(updated_content.contains("# --- Iris FZF Sync ---"));
            assert!(updated_content.contains("_iris_fzf_sync"));

            let result_second = IrisSetup::setup_zsh_hook(&ctx, &task);
            assert!(result_second.is_ok());

            let final_content = fs::read_to_string(&zshrc_path).unwrap();
            let occurrences = final_content.matches("# --- Iris FZF Sync ---").count();
            assert_eq!(occurrences, 1, "Hook should not be duplicated");
        });
    }

    #[test]
    fn should_skip_initial_setup_if_exists() {
        let (_tmp, mut ctx) = mock_context();

        fs::write(
            &ctx.paths.state_file,
            r#"{"current_theme": "nord", "enabled_generators": []}"#,
        )
        .unwrap();

        let task = ctx.log.step("Initial State Test", true);
        let result = IrisSetup::setup_initial_state(&mut ctx, &task);

        assert!(result.is_ok());
    }

    #[test]
    fn should_handle_full_setup_logic() {
        let (tmp, mut ctx) = mock_context();
        let fake_home = tmp.path();

        fs::write(fake_home.join(".zshrc"), "").unwrap();

        temp_env::with_vars(
            [
                ("HOME", Some(fake_home.to_str().unwrap())),
                (
                    "XDG_CONFIG_HOME",
                    Some(fake_home.join(".config").to_str().unwrap()),
                ),
                (
                    "XDG_DATA_HOME",
                    Some(fake_home.join(".local/share").to_str().unwrap()),
                ),
            ],
            || {
                let result = IrisSetup::run(&mut ctx);
                assert!(result.is_ok(), "Setup run failed: {:?}", result.err());
                assert!(ctx.paths.config.exists(), "Config dir should be created");
                assert!(ctx.paths.cache.exists(), "Cache dir should be created");
                assert!(
                    ctx.paths.state_file.exists(),
                    "state.json should be created"
                );
                assert_eq!(ctx.state.manager, PluginManager::Default);

                let zshrc_content = fs::read_to_string(fake_home.join(".zshrc")).unwrap();
                assert!(zshrc_content.contains("Iris FZF Sync"));
            },
        );
    }
}
