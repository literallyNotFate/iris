use crate::{
    cli::{GenAction, StatusFilter},
    core::IrisContext,
};
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
            let all_generators = ctx.registry.all();
            let mut items = Vec::new();
            let mut defaults = Vec::new();

            for generator in &all_generators {
                let name = generator.name();
                let g_type = generator.generator_type();

                let display_name = if generator.is_installed() {
                    format!(
                        "{:<14} {} ({})",
                        name,
                        g_type.icon().color(g_type.color()),
                        g_type.label()
                    )
                } else {
                    format!("{} {:<14} (not found)", "󰂭".bright_red(), name)
                };

                items.push(display_name);
                defaults.push(ctx.state.is_enabled(name));
            }

            println!(
                "\n {}  {}",
                "󰒓".green().bold(),
                "Generator Management".bold()
            );
            println!();

            let theme: ColorfulTheme = ColorfulTheme {
                active_item_prefix: style("  ❯ ".to_string()).for_stderr().cyan().bold(),
                checked_item_prefix: style("  󰄬 ".to_string()).for_stderr().green().bold(),
                unchecked_item_prefix: style("  󰄱 ".to_string()).for_stderr().dim(),
                active_item_style: Style::new().cyan().bold(),
                prompt_prefix: style("  ? ".to_string()).for_stderr().yellow(),
                prompt_suffix: style("".to_string()),
                inactive_item_prefix: style("    ".to_string()).for_stderr(),
                ..ColorfulTheme::default()
            };

            let chosen = MultiSelect::with_theme(&theme)
                .with_prompt(format!(
                    "Toggle modules ({}:toggle / {}:confirm)\n",
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

            {
                let mut task = ctx.log.step("Saving settings", 1);
                ctx.save()?;
                task.done(true);
            }

            if !selected_names.is_empty() {
                let list = selected_names
                    .iter()
                    .map(|n| n.cyan().to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                println!(" {} {} {}", "󱐋".green().bold(), "Active:".bold(), list);
            }
        }

        GenAction::Enable { name } => {
            let g = ctx.registry.get(&name).ok_or_else(|| {
                anyhow::anyhow!(
                    "Unknown generator: `{}`. Use `{}` to see available.",
                    name.bold().green(),
                    "iris gen list".italic().cyan()
                )
            })?;
            println!();

            if !g.is_installed() {
                ctx.log.warn(
                    &format!(
                        "Generator `{}` is recognized, but the app is not found in your OS",
                        name.bold().green()
                    ),
                    1,
                );
            }

            if ctx.state.enable_generator(&name) {
                let mut task = ctx.log.step(&format!("Enabling: {}", name.cyan()), 1);
                ctx.save()?;
                task.done(true);
            } else {
                ctx.log
                    .warn(&format!("`{}` is already active", name.bold().cyan()), 0);
            }
        }

        GenAction::Disable { name } => {
            let _ = ctx.registry.get(&name).ok_or_else(|| {
                anyhow::anyhow!(
                    "Unknown generator: `{}`. Use `{}` to see available.",
                    name.bold().green(),
                    "iris gen list".italic().cyan()
                )
            })?;
            println!();

            if ctx.state.disable_generator(&name) {
                let mut task = ctx.log.step(&format!("Disabling: {}", name.cyan()), 1);
                ctx.save()?;
                task.done(true);
            } else {
                ctx.log
                    .warn(&format!("`{}` is already disabled", name.cyan().bold()), 0);
            }
        }

        GenAction::List {
            generator_type,
            status,
        } => {
            let all_generators = ctx.registry.all();

            let filtered: Vec<_> = all_generators
                .into_iter()
                .filter(|g| {
                    let type_match = generator_type.map_or(true, |t| g.generator_type() == t);

                    let is_enabled = ctx.state.is_enabled(g.name());
                    let is_installed = g.is_installed();
                    let status_match = status.map_or(true, |s| match s {
                        StatusFilter::Active => is_enabled && is_installed,
                        StatusFilter::Ready => !is_enabled && is_installed,
                        StatusFilter::Broken => is_enabled && !is_installed,
                        StatusFilter::Missing => !is_enabled && !is_installed,
                    });

                    type_match && status_match
                })
                .collect();

            let total = filtered.len();
            let enabled_count = filtered
                .iter()
                .filter(|g| ctx.state.is_enabled(g.name()))
                .count();

            println!();
            if !ctx.log.quiet {
                println!(
                    " {}  {} {}",
                    "󰒓".yellow().bold(),
                    "Registry of Generators".bold(),
                    if generator_type.is_some() || status.is_some() {
                        "(filtered)".dimmed().italic()
                    } else {
                        "".into()
                    }
                );

                println!(
                    "\n    {:<20}  {:<14}  {}",
                    "NAME".dimmed(),
                    "TYPE".dimmed(),
                    "STATUS".dimmed()
                );
                println!();
            }

            for generator in &filtered {
                let is_enabled = ctx.state.is_enabled(generator.name());
                let is_installed = generator.is_installed();
                let gen_type = generator.generator_type();

                let icon = match (is_enabled, is_installed) {
                    (true, true) => "󰄬 ".green(),
                    (true, false) => "󰀦 ".yellow(),
                    (false, true) => "󰈈 ".dimmed(),
                    (false, false) => "󰂭 ".red(),
                };

                let name_styled = if is_enabled && is_installed {
                    generator.name().cyan().bold()
                } else if is_enabled && !is_installed {
                    generator.name().yellow().strikethrough()
                } else {
                    generator.name().normal()
                };

                if ctx.log.quiet {
                    let q_status = if is_enabled { "+".bold() } else { "-".dimmed() };
                    println!(
                        "{} {:<14} ({})",
                        q_status,
                        generator.name(),
                        gen_type.label()
                    );
                } else {
                    let status_label = match (is_enabled, is_installed) {
                        (true, true) => "active".green().italic(),
                        (false, true) => "ready".dimmed(),
                        (true, false) => "broken".yellow(),
                        (false, false) => "missing".red(),
                    };

                    let type_icon = gen_type.icon().color(gen_type.color());
                    let type_label = gen_type.label().color(gen_type.color());

                    println!(
                        "  {} {:<17} │ {} {:<11} │ {}",
                        icon, name_styled, type_icon, type_label, status_label
                    );
                }
            }

            if !ctx.log.quiet {
                println!(
                    "{}",
                    " ──────────────────────────────────────────────────────────".dimmed()
                );
            }

            if ctx.log.quiet {
                if total > 0 {
                    println!("\nTotal: {} (Enabled: {})", total, enabled_count);
                } else {
                    println!("No generators found.");
                }
            } else {
                println!(
                    " {} {} {} {} {} {}",
                    "󰛵".blue(),
                    "Showing:".dimmed(),
                    total.to_string().bold().cyan(),
                    "generators,".dimmed(),
                    enabled_count.to_string().bold().green(),
                    "enabled".dimmed()
                );

                if total == 0 {
                    println!(
                        " {} {}",
                        "⚠".yellow(),
                        "No generators match your filter criteria".italic().dimmed()
                    );
                } else if enabled_count == 0 && status.is_none() {
                    println!(
                        " {} {}",
                        "󰚔".yellow(),
                        "Tip: use `iris gen enable <name>` to start syncing configs".dimmed()
                    );
                }
            }
        }

        GenAction::Auto => {
            println!(
                "\n {}  {}",
                "󰩊".blue().bold(),
                "Autodiscovering generators...".bold()
            );
            println!();

            let mut added = 0;
            let installed = ctx.registry.installed();

            for generator in installed {
                let name = generator.name();
                if ctx.state.is_enabled(name) {
                    if !ctx.log.quiet {
                        ctx.log
                            .info(&format!("`{}` is already active", name.dimmed()));
                    }
                } else {
                    {
                        let mut task = ctx
                            .log
                            .step(&format!("Detected: {}", name.cyan().bold()), 1);
                        ctx.state.enable_generator(name);
                        added += 1;
                        task.done(true);
                    }
                }
            }

            if !ctx.log.quiet {
                println!();
            }

            if added > 0 {
                {
                    let mut task = ctx.log.step("Saving configuration", 1);
                    ctx.save()?;
                    task.done(true);
                }

                println!();
                ctx.log.success(
                    &format!(
                        "Auto-discovery complete! Added {} new generators",
                        added.to_string().green().bold()
                    ),
                    0,
                );
            } else {
                let msg = if !ctx.log.quiet {
                    "System is up to date: no new applications found."
                } else {
                    "All discovered apps are already active."
                };

                println!(" {} {}", "ℹ".blue().bold(), msg.dimmed());
            }
        }
    }

    Ok(())
}
