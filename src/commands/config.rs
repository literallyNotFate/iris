use crate::{
    cli::ConfigAction,
    core::{IrisContext, ThemeOrchestrator},
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
            render_config_header("Current Configuration", "󰒓");
            println!("  {:<18} {}", "Plugin Manager:".dimmed(), ctx.state.manager);
            println!(
                "  {:<18} {}",
                "Current Theme:".dimmed(),
                utils::capitalize(&ctx.state.current_theme).green()
            );
            println!(
                "  {:<18} {}",
                "Fallback Theme:".dimmed(),
                utils::capitalize(&ctx.state.fallback_theme).magenta()
            );
            println!(
                "  {:<18} {}",
                "Config File:".dimmed(),
                utils::pretty_path(&ctx.paths.state_file).dimmed()
            );
        }

        ConfigAction::Nvim { manager, detect } => {
            render_config_header("Neovim Configuration", "⚙");
            let orchestrator = ThemeOrchestrator::new(&ctx.paths, &ctx.log);

            let selected = if detect || manager.is_some() {
                orchestrator.choose_manager(manager, detect)?
            } else {
                println!(
                    "{}  No flags provided. Choose your plugin manager manually:",
                    "ℹ".blue()
                );
                let managers: [PluginManager; 3] = PluginManager::all();

                let selection: usize = Select::with_theme(&select_theme())
                    .items(&managers)
                    .default(0)
                    .interact()?;

                managers[selection].clone()
            };

            if ctx.state.manager != selected {
                ctx.log
                    .info(&format!("Changing plugin manager to {}...", selected));

                ctx.state.manager = selected;
                changed = true;
            } else {
                println!(
                    "{}  Plugin manager is already set to {}",
                    "✓".green(),
                    ctx.state.manager
                );
            }
        }

        ConfigAction::Fallback { name } => {
            render_config_header("Fallback Configuration", "⚙");
            let orchestrator = ThemeOrchestrator::new(&ctx.paths, &ctx.log);
            let input_name: String = match name {
                Some(n) => n,
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

            if ctx.state.fallback_theme != theme.to_lowercase() {
                ctx.log.info(&format!(
                    "Selecting {} as a fallback...",
                    theme.magenta().bold()
                ));
                ctx.state.fallback_theme = theme.to_lowercase();
                changed = true;
                println!("{}  Fallback theme updated successfully!", "✓".green());
            } else {
                println!(
                    "{}  Fallback theme is already set to {}",
                    "✓".green(),
                    theme.magenta().bold()
                );
            }
        }
    }

    if changed {
        println!();
        ctx.log.action("Saved configuration to state.json\n", || {
            ctx.state.save_to(&ctx.paths.state_file)
        })?;
        println!();
    } else {
        println!(
            "\n{}  No changes detected, state file left untouched.\n",
            "ℹ".blue()
        );
    }

    Ok(())
}

/// Helper function to render header for config
fn render_config_header(title: &str, icon: &str) {
    println!("\n{}  {}\n", icon.bright_yellow().bold(), title.bold());
}
