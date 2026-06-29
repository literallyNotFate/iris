use crate::{
    cli::ConfigAction,
    core::{IrisContext, ThemeOrchestrator},
    log::LoggingVerbosity,
    models::PluginManager,
    utils::{self, colors::select_theme},
};
use colored::*;
use dialoguer::{Input, Select};

/// Handle application config command
pub fn exec(action: Option<ConfigAction>, ctx: &mut IrisContext) -> anyhow::Result<()> {
    let mut changed: bool = false;
    let action: ConfigAction = action.unwrap_or(ConfigAction::Show);

    match action {
        ConfigAction::Show => {
            if ctx.log.is_detailed() {
                render_config_header("Current Configuration", "󰒓");
                println!(
                    "  {:<18} {}",
                    "Plugin Manager:".dimmed(),
                    ctx.state.nvim.manager
                );
                println!(
                    "  {:<18} {}",
                    "Current Theme:".dimmed(),
                    utils::capitalize(&ctx.state.theme.current_theme).green()
                );
                println!(
                    "  {:<18} {}",
                    "Fallback Theme:".dimmed(),
                    utils::capitalize(&ctx.state.theme.fallback_theme).magenta()
                );
                println!(
                    "  {:<18} {}",
                    "Config File:".dimmed(),
                    utils::pretty_path(&ctx.paths.state_file).dimmed()
                );
            } else if ctx.log.verbosity == LoggingVerbosity::Minimal {
                println!(
                    "\n󰒓  Config: Active: {} | Fallback: {} | Manager: {}",
                    utils::capitalize(&ctx.state.theme.current_theme)
                        .green()
                        .bold(),
                    utils::capitalize(&ctx.state.theme.fallback_theme)
                        .magenta()
                        .bold(),
                    ctx.state.nvim.manager
                );
            }
        }

        ConfigAction::Nvim { manager, detect } => {
            if ctx.log.is_detailed() {
                render_config_header("Neovim Configuration", "⚙");
            }
            let orchestrator = ThemeOrchestrator::new(&ctx.paths, &ctx.log);

            let selected: PluginManager = if detect || manager.is_some() {
                orchestrator.choose_manager(manager, detect)?
            } else {
                if ctx.log.verbosity != LoggingVerbosity::Silent {
                    println!(
                        "{}  No flags provided. Choose your plugin manager manually:",
                        "ℹ".blue()
                    );
                }

                let managers: [PluginManager; 3] = PluginManager::all();
                let selection: usize = Select::with_theme(&select_theme())
                    .items(&managers)
                    .default(0)
                    .interact()?;

                managers[selection].clone()
            };

            if ctx.state.nvim.manager != selected {
                if ctx.log.is_detailed() {
                    ctx.log
                        .info(&format!("Changing plugin manager to {}...", selected));
                }

                ctx.state.nvim.manager = selected;
                changed = true;
            } else if ctx.log.verbosity != LoggingVerbosity::Silent {
                println!(
                    "{}  Plugin manager is already set to {}",
                    "✓".green(),
                    ctx.state.nvim.manager
                );
            }
        }

        ConfigAction::Fallback { ref name } => {
            if ctx.log.is_detailed() {
                render_config_header("Fallback Configuration", "⚙");
            }

            let orchestrator = ThemeOrchestrator::new(&ctx.paths, &ctx.log);
            let input_name: String = match name {
                Some(n) => n.to_owned(),
                None => Input::<String>::with_theme(&select_theme())
                    .with_prompt("Enter fallback theme name:")
                    .interact_text()?,
            };

            let theme: String = utils::capitalize(input_name.trim());
            if !orchestrator.theme_exists(&theme, &ctx.state) {
                anyhow::bail!(
                    "Theme `{}` does not exist in Neovim or cache.",
                    theme.cyan().bold()
                );
            }

            if ctx.state.theme.fallback_theme != theme.to_lowercase() {
                if ctx.log.is_detailed() {
                    ctx.log.info(&format!(
                        "Selecting {} as a fallback...",
                        theme.magenta().bold()
                    ));
                }

                ctx.state.theme.fallback_theme = theme.to_lowercase();
                changed = true;
                if ctx.log.verbosity != LoggingVerbosity::Silent {
                    println!(
                        "\n{}",
                        "✓ Fallback theme updated successfully!".green().bold()
                    );
                }
            } else if ctx.log.verbosity != LoggingVerbosity::Silent {
                println!(
                    "\n{} {}",
                    "✓ Fallback theme is already set to".green().bold(),
                    theme.magenta().bold()
                );
            }
        }
    }

    if changed {
        ctx.log.action("Saved configuration to state file", || {
            ctx.state.save_to(&ctx.paths.state_file)
        })?;
    } else if !matches!(action, ConfigAction::Show) && ctx.log.is_detailed() {
        println!(
            "{}  No changes detected, state file left untouched.",
            "ℹ".blue()
        );
    }

    if ctx.log.verbosity != LoggingVerbosity::Silent {
        println!();
    }

    Ok(())
}

/// Helper function to render header for config
fn render_config_header(title: &str, icon: &str) {
    println!("\n{}  {}\n", icon.bright_yellow().bold(), title.bold());
}
