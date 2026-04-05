use crate::{cli::GenAction, core::IrisContext};
use colored::{Color, Colorize};
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
                let display_name = if generator.is_installed() {
                    name.to_string()
                } else {
                    format!("{} {}", name, "(not found)".red().dimmed())
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
            println!();

            if !ctx.registry.exists(&name) {
                ctx.log.error(
                    &format!(
                        "Unknown generator: '{}'. Use 'iris gen list' to see available.",
                        name.bold()
                    ),
                    0,
                );
                return Ok(());
            }

            if !ctx.registry.is_installed(&name) {
                ctx.log.warn(
                    "Generator exists in Iris, but the app is not installed in your OS",
                    0,
                );
            }

            if ctx.state.enable_generator(&name) {
                let mut task = ctx.log.step(&format!("Enabling: {}", name.cyan()), 0);
                ctx.save()?;
                task.done(true);
            } else {
                ctx.log
                    .warn(&format!("'{}' is already active", name.bold()), 0);
            }
        }

        GenAction::Disable { name } => {
            println!();

            if !ctx.registry.exists(&name) {
                ctx.log.error(
                    &format!(
                        "Unknown generator: '{}'. Use 'iris gen list' to see available.",
                        name.bold()
                    ),
                    0,
                );
                return Ok(());
            }

            if ctx.state.disable_generator(&name) {
                let mut task = ctx.log.step(&format!("Disabling: {}", name.cyan()), 0);
                ctx.save()?;
                task.done(true);
            } else {
                ctx.log
                    .warn(&format!("'{}' is already disabled", name.bold()), 0);
            }
        }

        GenAction::List => {
            let all = ctx.registry.all();
            let total = all.len();
            let enabled_count = all
                .iter()
                .filter(|g| ctx.state.is_enabled(g.name()))
                .count();

            println!();
            if !ctx.log.quiet {
                println!(
                    " {}  {}",
                    "󰒓".yellow().bold(),
                    "Registry of Generators".bold()
                );

                println!(
                    "\n    {:<17}  {:<13}  {}",
                    "NAME".dimmed(),
                    "TYPE".dimmed(),
                    "STATUS".dimmed()
                );
                println!();
            }

            for generator in ctx.registry.all() {
                let is_enabled = ctx.state.is_enabled(generator.name());
                let is_installed = generator.is_installed();

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

                let (tag_icon, tag_text, tag_color) = match generator.name() {
                    "ghostty" | "alacritty" => ("󰞷", "term", Color::Blue),
                    "bat" | "fzf" | "yazi" => ("󰆍", "cli", Color::Magenta),
                    "nvim" => ("", "edit", Color::Green),
                    "starship" => ("󱆃", "prompt", Color::Cyan),
                    "btop" => ("󰢮", "sys", Color::Yellow),
                    _ => ("󰏗", "app", Color::White),
                };

                let tag_styled = format!("{} {}", tag_icon, tag_text).color(tag_color);

                if ctx.log.quiet {
                    let q_status = if is_enabled { "+" } else { "-" };
                    println!("{} {:<12} ({})", q_status, generator.name(), tag_text);
                } else {
                    let status_label = match (is_enabled, is_installed) {
                        (true, true) => "active".green().italic(),
                        (false, true) => "ready".dimmed(),
                        (true, false) => "broken".yellow(),
                        (false, false) => "missing".red(),
                    };

                    println!(
                        "  {} {:<16} │ {:<12} │ {}",
                        icon, name_styled, tag_styled, status_label
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
                println!("\nTotal: {} (Enabled: {})", total, enabled_count);
            } else {
                println!(
                    " {} {} {} {} {} {}",
                    "󰛵".blue(),
                    "Found:".dimmed(),
                    total.to_string().bold().cyan(),
                    "generators,".dimmed(),
                    enabled_count.to_string().bold().green(),
                    "enabled".dimmed()
                );

                if enabled_count == 0 {
                    println!(
                        " {} {}",
                        "󰚔".yellow(),
                        "Tip: use 'iris gen enable <name>' to start syncing configs"
                            .italic()
                            .dimmed()
                    );
                }
                println!();
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
                            .info(&format!("{} is already active", name.dimmed()));
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
