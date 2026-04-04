use crate::{cli::GenAction, core::IrisContext, utils::Status};
use colored::Colorize;
use dialoguer::{
    MultiSelect,
    console::{Style, style},
    theme::ColorfulTheme,
};
use std::collections::BTreeSet;

/// Handle application gen command and its subcommands
pub fn exec(action: GenAction, ctx: &mut IrisContext) -> anyhow::Result<()> {
    match action {
        GenAction::Select => {
            let all_generators = &ctx.registry.generators;
            let mut items = Vec::new();
            let mut defaults = Vec::new();

            for generator in all_generators {
                let name = generator.name();
                items.push(if generator.is_installed() {
                    name.to_string()
                } else {
                    format!("{} {}", name, "(not found)".red().dimmed())
                });
                defaults.push(ctx.state.is_enabled(name));
            }

            println!(
                "\n {}  {}",
                "󰒓".green().bold(),
                "Generator Management".bold()
            );
            println!("{}", " ─────────────────────────────────────────".dimmed());
            println!();

            let theme = ColorfulTheme {
                checked_item_prefix: style("󰄬".to_string()).for_stderr().green().bold(),
                unchecked_item_prefix: style("󰄱".to_string()).for_stderr().dim(),
                active_item_style: Style::new().cyan().bold(),
                ..ColorfulTheme::default()
            };

            let chosen = MultiSelect::with_theme(&theme)
                .with_prompt(format!(
                    "Toggle modules ({}:toggle / {}:confirm)",
                    "space".yellow(),
                    "enter".cyan()
                ))
                .items(&items)
                .defaults(&defaults)
                .report(false)
                .interact()?;

            let selected_names: BTreeSet<String> = chosen
                .iter()
                .map(|&i| all_generators[i].name().to_string())
                .collect();

            ctx.state.replace_enabled(selected_names.clone());

            let task = Status::step("Saving settings...", 0);
            ctx.save()?;
            task.done(Some("state.json updated"));

            if !selected_names.is_empty() {
                let list = selected_names
                    .iter()
                    .map(|n| n.cyan().to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                println!("\n {} {} {}", "󰄬".green(), "Active:".bold(), list);
            }
        }

        GenAction::Enable { name } => {
            println!();

            if !ctx.registry.exists(&name) {
                Status::error(
                    &format!(
                        "Unknown generator: '{}'. Use 'iris gen list' to see available.",
                        name.bold()
                    ),
                    0,
                );
                return Ok(());
            }

            if !ctx.registry.is_installed(&name) {
                Status::warn(
                    "Generator exists in Iris, but the app is not installed in your OS",
                    0,
                );
            }

            if ctx.state.enable_generator(&name) {
                let task = Status::step(&format!("Enabling: {}", name.cyan()), 0);
                ctx.save()?;
                task.done(Some("Enabled"));
            } else {
                Status::warn(&format!("'{}' is already active", name.bold()), 0);
            }
        }

        GenAction::Disable { name } => {
            println!();

            if !ctx.registry.exists(&name) {
                Status::error(
                    &format!(
                        "Unknown generator: '{}'. Use 'iris gen list' to see available.",
                        name.bold()
                    ),
                    0,
                );
                return Ok(());
            }

            if ctx.state.disable_generator(&name) {
                let task = Status::step(&format!("Disabling: {}", name.cyan()), 0);
                ctx.save()?;
                task.done(Some("Disabled"));
            } else {
                Status::warn(&format!("'{}' is already disabled", name.bold()), 0);
            }
        }

        GenAction::List => {
            println!(
                "\n {}  {}",
                "󰈙".yellow().bold(),
                "Available generators:".bold()
            );
            println!("{}", " ──────────────────────────────".dimmed());

            for generator in ctx.registry.all() {
                let (icon, status) = match (
                    ctx.state.is_enabled(generator.name()),
                    generator.is_installed(),
                ) {
                    (true, true) => ("󰄬".green(), "active".green()),
                    (false, true) => ("󰈈".dimmed(), "disabled".dimmed()),
                    (true, false) => ("󰀦".yellow(), "broken (not installed)".yellow()),
                    (false, false) => ("󰂭".red(), "missing".red()),
                };

                println!("  {} {:<12} [{}]", icon, generator.name().bold(), status);
            }

            println!();
        }

        GenAction::Auto => {
            println!(
                "\n {}  {}",
                "󰩊".blue().bold(),
                "Autodiscovering generators...".bold()
            );
            println!("{}", " ─────────────────────────────────────────".dimmed());

            let mut added = 0;
            let mut active = 0;

            for generator in ctx.registry.installed() {
                let name = generator.name();

                if ctx.state.is_enabled(name) {
                    active += 1;
                    Status::warn(&format!("{} is already active", name), 0);
                } else {
                    let task = Status::step(&format!("Found: {}", name.green()), 0);
                    ctx.state.enable_generator(name);
                    added += 1;
                    task.done(Some(&format!("Generator '{}' enabled", name.cyan().bold())));
                }
            }

            println!();

            if added > 0 {
                let task = Status::step("Finalizing changes", 0);
                ctx.save()?;
                task.done(Some(&format!(
                    "Added {} new generators to configuration",
                    added.to_string().yellow().bold()
                )));
            } else if active > 0 {
                Status::warn(
                    "Everything installed is already enabled. No changes needed.",
                    0,
                );
            } else {
                Status::error("No compatible apps found on this system.", 0);
            }
        }
    }

    Ok(())
}
