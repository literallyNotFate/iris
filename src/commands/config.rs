use crate::{
    cli::{ConfigAction, config::ConfigKey},
    core::{IrisContext, ThemeOrchestrator},
    log::LoggingVerbosity,
    models::PluginManager,
    utils::{self, colors::select_theme},
};
use anyhow::{Context, Result};
use colored::*;
use dialoguer::{Input, Select};
use std::{env, process::Command};

/// Handle application config command
pub fn exec(action: Option<ConfigAction>, ctx: &mut IrisContext) -> Result<()> {
    let mut changed: bool = false;
    let action: ConfigAction = action.unwrap_or(ConfigAction::Show);
    let is_display_or_edit: bool = matches!(action, ConfigAction::Show | ConfigAction::Edit);

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

        ConfigAction::Set { key, value } => {
            let orchestrator = ThemeOrchestrator::new(&ctx.paths, &ctx.log);

            match key {
                ConfigKey::Manager => {
                    if ctx.log.is_detailed() {
                        render_config_header("Neovim Configuration", "⚙");
                    }

                    let selected: PluginManager = match value {
                        Some(val) => match val.trim().to_lowercase().as_str() {
                            "lazy" => PluginManager::Lazy,
                            "packer" => PluginManager::Packer,
                            "default" => PluginManager::Default,
                            _ => anyhow::bail!(
                                "Invalid manager type. Available: lazy, packer, default"
                            ),
                        },
                        None => {
                            if let Ok(detected) = orchestrator.choose_manager(None, true) {
                                if ctx.log.verbosity != LoggingVerbosity::Silent {
                                    println!(
                                        "{}  Auto-detected plugin manager: {}",
                                        "✓".green(),
                                        detected
                                    );
                                }
                                detected
                            } else {
                                if ctx.log.verbosity != LoggingVerbosity::Silent {
                                    println!(
                                        "{}  Could not auto-detect. Choose manually:",
                                        "ℹ".blue()
                                    );
                                }
                                let managers = PluginManager::all();
                                let selection = Select::with_theme(&select_theme())
                                    .items(&managers)
                                    .default(0)
                                    .interact()?;
                                managers[selection].clone()
                            }
                        }
                    };

                    if ctx.state.nvim.manager != selected {
                        if ctx.log.is_detailed() {
                            ctx.log.info(&format!(
                                "Changing plugin manager to {}...",
                                selected.to_string().cyan().bold()
                            ));
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

                ConfigKey::Fallback => {
                    if ctx.log.is_detailed() {
                        render_config_header("Fallback Configuration", "⚙");
                    }

                    let input_name = match value {
                        Some(val) => val,
                        None => Input::<String>::with_theme(&select_theme())
                            .with_prompt("Enter fallback theme name:")
                            .interact_text()?,
                    };

                    let theme = utils::capitalize(input_name.trim());
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
        }

        ConfigAction::Edit => {
            println!();
            let editor = env::var("EDITOR").unwrap_or_else(|_| "nvim".to_string());

            if ctx.log.is_detailed() {
                ctx.log.info(&format!(
                    "Opening {} in {}...",
                    utils::pretty_path(&ctx.paths.state_file).cyan().italic(),
                    editor.green().bold()
                ));
            }

            let status = Command::new(&editor)
                .arg(&ctx.paths.state_file)
                .status()
                .with_context(|| format!("Failed to run editor: '{editor}'"))?;

            if !status.success() {
                anyhow::bail!("Editor exited with a non-zero status code.");
            }
        }
    }

    if changed {
        ctx.log.action("Saved configuration to state file", || {
            ctx.state.save_to(&ctx.paths.state_file)
        })?;
    } else if !is_display_or_edit && ctx.log.is_detailed() {
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
