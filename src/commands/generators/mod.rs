pub(crate) mod ui;

use crate::{
    cli::GenAction,
    core::IrisContext,
    modules::{GeneratorType, StateFilter},
    utils,
};
use colored::Colorize;
use dialoguer::MultiSelect;
use std::collections::BTreeSet;

/// Handle application gen command and its subcommands
pub fn exec(action: GenAction, ctx: &mut IrisContext) -> anyhow::Result<()> {
    match action {
        GenAction::Select => handle_select(ctx)?,
        GenAction::Enable { name } => handle_status_change(name, true, ctx)?,
        GenAction::Disable { name } => handle_status_change(name, false, ctx)?,
        GenAction::Auto => handle_auto(ctx)?,
        GenAction::List {
            generator_type,
            status,
        } => render_list(generator_type, status, ctx)?,
    }
    Ok(())
}

/// Select generators using dialoguer
fn handle_select(ctx: &mut IrisContext) -> anyhow::Result<()> {
    let all_generators = ctx.registry.all();
    let mut selectable_generators = Vec::new();
    let mut missing_generators = Vec::new();

    for g in &all_generators {
        if g.is_installed() {
            selectable_generators.push(g);
        } else {
            missing_generators.push(g);
        }
    }

    let mut items = Vec::new();
    let mut defaults = Vec::new();

    for g in &selectable_generators {
        let name = g.name();
        let g_type = g.generator_type();

        let display = format!(
            "{:<14} {} ({})",
            name,
            g_type.icon().color(g_type.color()),
            g_type.label()
        );

        items.push(display);
        defaults.push(ctx.state.is_enabled(name));
    }

    ui::render_header("Generator Management", "󰒓");

    let chosen: Vec<usize> = if items.is_empty() {
        ctx.log
            .warn("No supported terminal emulators or utilities found in your system.");
        Vec::new()
    } else {
        MultiSelect::with_theme(&utils::colors::select_theme())
            .with_prompt(format!(
                "Toggle modules ({}:toggle / {}:confirm)\n",
                "space".yellow(),
                "enter".cyan()
            ))
            .items(&items)
            .defaults(&defaults)
            .report(false)
            .interact()?
    };

    let mut selected_names: BTreeSet<String> = chosen
        .iter()
        .map(|&i| selectable_generators[i].name().to_string())
        .collect();

    for g in &missing_generators {
        if ctx.state.is_enabled(g.name()) {
            selected_names.insert(g.name().to_string());
        }
    }

    ctx.state.replace_enabled(selected_names.clone());

    let success_msg: &str = if !selected_names.is_empty() {
        &format!(
            "Settings updated and saved!\n  Active modules: {}\n",
            ctx.registry.active(&ctx.state)
        )
    } else {
        "Settings updated and saved!\n"
    };

    ctx.log.action(success_msg, || ctx.save())?;

    if !missing_generators.is_empty() {
        println!("{}", "Unavailable modules (not installed):".dimmed());
        for g in missing_generators {
            println!(
                "  {}  {:<14} {}",
                "󰂭".dimmed(),
                g.name().dimmed(),
                "(not found)".red().dimmed()
            );
        }
        println!();
    }

    Ok(())
}

/// Enabling/disabling generator
fn handle_status_change(name: String, enable: bool, ctx: &mut IrisContext) -> anyhow::Result<()> {
    let _ = ctx.resolve_generator(&name)?;
    println!();

    let success = if enable {
        if !ctx.registry.get(&name).unwrap().is_installed() {
            ctx.log.warn(&format!(
                "Generator `{}` is recognized, but app is not found!",
                name.green()
            ));
        }
        ctx.state.enable_generator(&name)
    } else {
        ctx.state.disable_generator(&name)
    };

    if success {
        let verb = if enable { "Enabling" } else { "Disabling" };
        let list = ctx.registry.active(&ctx.state);

        let success_msg: &str = if !list.is_empty() {
            &format!(
                "{} generator: {}\n  Active modules: {}",
                verb,
                name.cyan(),
                list
            )
        } else {
            &format!("{} generator: {}", verb, name.cyan())
        };

        ctx.log.action(success_msg, || ctx.save())?;
    } else {
        let status = if enable {
            "already active"
        } else {
            "already disabled"
        };

        ctx.log
            .warn(&format!("`{}` is {}", name.cyan().bold(), status));
    }

    println!();
    Ok(())
}

/// Autodiscovering generators
pub fn handle_auto(ctx: &mut IrisContext) -> anyhow::Result<()> {
    ui::render_header("Autodiscovering generators...", "󰩊");

    let mut added: i32 = 0;
    let all_generators = ctx.registry.all();

    ctx.log
        .info("Scanning system for supported applications...");

    for g in &all_generators {
        if g.is_installed() && !ctx.state.is_enabled(g.name()) {
            ctx.state.enable_generator(g.name());
            println!("    Detected {}, enabling...", g.name().green().bold());
            added += 1;
        }
    }

    if added > 0 {
        println!(
            "{} {} {} {}",
            "└──".dimmed(),
            "Added".bold(),
            format!("{} new generators to configuration", added)
                .cyan()
                .bold(),
            "✓".green()
        );

        println!();
        ctx.log
            .action("Saved configuration to state file", || ctx.save())?;
    } else {
        ctx.log.info("All discovered apps are already active.");
    }

    println!();
    Ok(())
}

/// Render generators list
fn render_list(
    gen_type: Option<GeneratorType>,
    status_filter: Option<StateFilter>,
    ctx: &IrisContext,
) -> anyhow::Result<()> {
    println!();

    let filtered: Vec<_> = ctx
        .registry
        .all()
        .into_iter()
        .filter(|g| {
            let is_enabled = ctx.state.is_enabled(g.name());
            let is_installed = g.is_installed();

            let type_match = gen_type.map_or(true, |t| g.generator_type() == t);
            let status_match = status_filter.map_or(true, |f| f.matches(is_enabled, is_installed));

            type_match && status_match
        })
        .collect();

    let total = filtered.len();
    let enabled_count = filtered
        .iter()
        .filter(|g| ctx.state.is_enabled(g.name()))
        .count();

    if ctx.log.is_detailed() {
        ui::render_list_header(gen_type.is_some() || status_filter.is_some());
    }

    for g in filtered {
        let is_enabled = ctx.state.is_enabled(g.name());
        let is_installed = g.is_installed();

        if !ctx.log.is_detailed() {
            ui::render_quiet_row(g, is_enabled);
        } else {
            ui::render_full_row(g, is_enabled, is_installed);
        }
    }

    if ctx.log.is_detailed() {
        ui::render_list_footer(total, enabled_count, status_filter.is_none());
    } else if total > 0 {
        println!("\nTotal: {} (Enabled: {})\n", total, enabled_count);
    }

    Ok(())
}
