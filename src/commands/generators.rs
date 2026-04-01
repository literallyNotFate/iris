use crate::{cli::GenAction, core::IrisContext, utils::Status};
use colored::Colorize;
use dialoguer::{MultiSelect, theme::ColorfulTheme};

/// Handle application gen command and its subcommands
pub fn exec(action: GenAction, ctx: &mut IrisContext) -> anyhow::Result<()> {
    match action {
        GenAction::Select => {
            let all_apps: Vec<String> = crate::modules::all_generators()
                .iter()
                .map(|x| x.name().to_string())
                .collect();

            let defaults: Vec<bool> = all_apps
                .iter()
                .map(|app| ctx.state.enabled_generators.contains(app))
                .collect();

            println!(
                "\n {}  {}",
                "󰒓".green().bold(),
                "Configuration: Generator Management".bold()
            );
            println!("{}", " ─────────────────────────────────────────".dimmed());

            let prompt = format!(
                "Toggle generators ({}:toggle / {}:confirm)",
                "space".purple(),
                "enter".cyan()
            );

            let chosen = MultiSelect::with_theme(&ColorfulTheme::default())
                .with_prompt(prompt)
                .items(&all_apps)
                .defaults(&defaults)
                .report(false)
                .interact()?;

            ctx.state.enabled_generators.clear();

            if !chosen.is_empty() {
                println!("\n {} {}", "󰄬".green(), "Active modules:".bold());
                for &idx in &chosen {
                    let name = all_apps[idx].clone();
                    println!("   {} {}", "󰄬".green(), name.dimmed());
                    ctx.state.enabled_generators.insert(name);
                }
            } else {
                println!(
                    "\n {}  {}",
                    "󰔟".yellow(),
                    "No generators selected.".yellow()
                );
            }

            println!();
            let task = Status::step("Applying configuration", 0);
            ctx.save()?;
            task.done(Some("state.json updated successfully"));
        }

        GenAction::Enable { name } => {
            println!();
            let exists = crate::modules::all_generators()
                .iter()
                .any(|g| g.name() == name);

            if !exists {
                Status::error(
                    &format!(
                        "Unknown generator: '{}'. Use 'iris gen list' to see available.",
                        name.bold()
                    ),
                    0,
                );
                return Ok(());
            }

            if ctx.state.enabled_generators.contains(&name) {
                Status::warn(
                    &format!("Generator '{}' is already active.", name.bold()),
                    0,
                );
                return Ok(());
            }

            let task = Status::step(&format!("Enabling generator: {}", name.cyan()), 0);
            ctx.state.enabled_generators.insert(name.clone());

            match ctx.save() {
                Ok(_) => task.done(Some(&format!("Generator '{}' is now active", name))),
                Err(e) => task.fail(&format!("Failed to update state: {}", e)),
            }
        }

        GenAction::Disable { name } => {
            println!();
            let exists = crate::modules::all_generators()
                .iter()
                .any(|g| g.name() == name);

            if !exists {
                Status::error(
                    &format!(
                        "Unknown generator: '{}'. Use 'iris gen list' to see available.",
                        name.bold()
                    ),
                    0,
                );
                return Ok(());
            }

            if !ctx.state.enabled_generators.contains(&name) {
                Status::warn(
                    &format!("Generator '{}' is already disabled.", name.bold()),
                    0,
                );
                return Ok(());
            }

            let task = Status::step(&format!("Disabling generator: {}", name.cyan()), 0);
            ctx.state.enabled_generators.remove(&name);

            match ctx.save() {
                Ok(_) => task.done(Some(&format!("Generator '{}' has been disabled", name))),
                Err(e) => task.fail(&format!("Failed to update state: {}", e)),
            }
        }

        GenAction::List => {
            println!(
                "\n {}  {}",
                "󰈙".yellow().bold(),
                "Available generators:".bold()
            );
            println!("{}", " ──────────────────────────────".dimmed());

            for generator in crate::modules::all_generators() {
                let name = generator.name();
                let is_enabled = ctx.state.enabled_generators.contains(name);
                let is_installed = generator.is_installed();

                let (icon, status) = match (is_enabled, is_installed) {
                    (true, true) => ("󰄬".green(), "active".green()),
                    (false, true) => ("󰈈".dimmed(), "disabled".dimmed()),
                    (true, false) => ("󰀦".yellow(), "enabled but not installed".yellow()),
                    (false, false) => ("󰂭".red(), "not installed".red()),
                };

                println!("  {} {:<12} [{}]", icon, name.bold(), status);
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

            for generator in crate::modules::all_generators() {
                let name = generator.name();
                if generator.is_installed() {
                    if !ctx.state.enabled_generators.contains(name) {
                        let task = Status::step(&format!("Found {}", name.cyan()), 0);
                        ctx.state.enabled_generators.insert(name.to_string());
                        added += 1;
                        task.done(Some(&format!("Generator '{}' enabled", name)));
                    } else {
                        active += 1;
                        Status::warn(&format!("{} is already active", name), 0);
                    }
                }
            }

            println!();
            if added > 0 {
                let task = Status::step("Finalizing changes", 0);
                ctx.save()?;
                task.done(Some(&format!(
                    "Added {} new generators to configuration",
                    added
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
