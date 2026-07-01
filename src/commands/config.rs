use crate::{
    cli::{ConfigAction, config::ConfigKey},
    core::{IrisContext, ThemeOrchestrator},
    log::LoggingVerbosity,
    models::{PluginManager, State},
    utils::{self, colors::select_theme},
};
use anyhow::{Context, Result};
use colored::*;
use dialoguer::{Confirm, Input, Select};
use std::{env, process::Command};

/// Handle application config command
pub fn exec(action: Option<ConfigAction>, ctx: &mut IrisContext) -> Result<()> {
    let mut changed: bool = false;
    let action: ConfigAction = action.unwrap_or(ConfigAction::Show);
    let is_display_or_helper: bool = matches!(
        action,
        ConfigAction::Show | ConfigAction::Edit | ConfigAction::Check | ConfigAction::Reset
    );

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

                    let selected = resolve_manager(value, &orchestrator, ctx.log.verbosity)?;
                    changed = ctx.state.set_nvim_manager(selected);

                    if changed {
                        if ctx.log.is_detailed() {
                            ctx.log.info(&format!(
                                "Changing plugin manager to {}...",
                                selected.to_string().cyan().bold()
                            ));
                        }
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

                    let raw_name: String = resolve_fallback_name(value)?;
                    changed = ctx.state.set_fallback_theme(&raw_name, &orchestrator)?;

                    if changed {
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
                            utils::capitalize(&ctx.state.theme.fallback_theme)
                                .magenta()
                                .bold()
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

        ConfigAction::Check => {
            render_config_header("Environment Health Check", "󰍉");
            let errors: usize = ctx.paths.check_health(ctx.log.verbosity);

            println!();
            if errors == 0 {
                println!("{}", "✓ Iris environment is healthy.".green().bold());
            } else {
                anyhow::bail!("Environment check failed with {} error(s).", errors);
            }
        }

        ConfigAction::Reset => {
            render_config_header("Reset Configuration", "!");

            println!(
                "{} This will restore default settings in {} and overwrite current modifications.",
                "!".yellow().bold(),
                utils::pretty_path(&ctx.paths.state_file).dimmed()
            );

            let proceed = Confirm::with_theme(&select_theme())
                .with_prompt("Are you absolutely sure you want to reset your config?")
                .default(false)
                .interact()?;

            if !proceed {
                println!("\n{} Reset aborted.\n", "ℹ".blue());
                return Ok(());
            }

            ctx.state = State::default();
            changed = true;

            println!(
                "\n{}",
                "✓ Configuration reset to factory defaults successfully!"
                    .green()
                    .bold()
            );
        }
    }

    if changed {
        ctx.log.action("Saved configuration to state file", || {
            ctx.state.save_to(&ctx.paths.state_file)
        })?;
    } else if !is_display_or_helper && ctx.log.is_detailed() {
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

/// Helper function to resolve plugin manager
fn resolve_manager(
    value: Option<String>,
    orchestrator: &ThemeOrchestrator,
    verbosity: LoggingVerbosity,
) -> Result<PluginManager> {
    if let Some(val) = value {
        return match val.trim().to_lowercase().as_str() {
            "lazy" => Ok(PluginManager::Lazy),
            "packer" => Ok(PluginManager::Packer),
            "default" => Ok(PluginManager::Default),
            _ => anyhow::bail!("Invalid manager type. Available: lazy, packer, default"),
        };
    }

    if let Ok(detected) = orchestrator.choose_manager(None, true) {
        if verbosity != LoggingVerbosity::Silent {
            println!(
                "{}  Auto-detected plugin manager: {}",
                "✓".green(),
                detected
            );
        }
        return Ok(detected);
    }

    if verbosity != LoggingVerbosity::Silent {
        println!("{}  Could not auto-detect. Choose manually:", "ℹ".blue());
    }
    let managers = PluginManager::all();
    let selection = Select::with_theme(&select_theme())
        .items(&managers)
        .default(0)
        .interact()?;
    Ok(managers[selection].clone())
}

/// Helper function to resolve fallback theme
fn resolve_fallback_name(value: Option<String>) -> Result<String> {
    match value {
        Some(val) => Ok(val),
        None => Ok(Input::<String>::with_theme(&select_theme())
            .with_prompt("Enter fallback theme name:")
            .interact_text()?),
    }
}
