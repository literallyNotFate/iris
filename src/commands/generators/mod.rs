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
    let mut items = Vec::new();
    let mut defaults = Vec::new();

    for g in &all_generators {
        let name = g.name();
        let g_type = g.generator_type();

        let display = if g.is_installed() {
            format!(
                "{:<14} {} ({})",
                name,
                g_type.icon().color(g_type.color()),
                g_type.label()
            )
        } else {
            format!("{} {:<14} (not found)", "󰂭".bright_red(), name)
        };

        items.push(display);
        defaults.push(ctx.state.is_enabled(name));
    }

    ui::render_header("Generator Management", "󰒓");

    let chosen: Vec<usize> = MultiSelect::with_theme(&utils::colors::select_theme())
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

    let mut task = ctx.log.step("Saving settings", 1);
    ctx.save()?;
    task.done(true);

    if !selected_names.is_empty() {
        let list = selected_names
            .iter()
            .map(|n| n.cyan().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        println!(" {} {} {}", "󱐋".green().bold(), "Active:".bold(), list);
    }
    Ok(())
}

/// Enabling/disabling generator
fn handle_status_change(name: String, enable: bool, ctx: &mut IrisContext) -> anyhow::Result<()> {
    let _ = ctx.resolve_generator(&name)?;
    println!();

    let success = if enable {
        if !ctx.registry.get(&name).unwrap().is_installed() {
            ctx.log.warn(
                &format!(
                    "Generator `{}` is recognized, but app is not found",
                    name.green()
                ),
                1,
            );
        }
        ctx.state.enable_generator(&name)
    } else {
        ctx.state.disable_generator(&name)
    };

    if success {
        let action_msg = if enable { "Enabling" } else { "Disabling" };
        let mut task = ctx.log.step(&format!("{}: {}", action_msg, name.cyan()), 1);
        ctx.save()?;
        task.done(true);
    } else {
        let status_msg = if enable {
            "already active"
        } else {
            "already disabled"
        };
        ctx.log
            .warn(&format!("`{}` is {}", name.cyan().bold(), status_msg), 0);
    }
    Ok(())
}

/// Autodiscovering generators
fn handle_auto(ctx: &mut IrisContext) -> anyhow::Result<()> {
    ui::render_header("Autodiscovering generators...", "󰩊");

    let mut added = 0;
    for g in ctx.registry.installed() {
        if !ctx.state.is_enabled(g.name()) {
            let mut task = ctx
                .log
                .step(&format!("Detected: {}", g.name().cyan().bold()), 1);
            ctx.state.enable_generator(g.name());
            added += 1;
            task.done(true);
        }
    }

    if added > 0 {
        let mut task = ctx.log.step("Saving configuration", 1);
        ctx.save()?;
        task.done(true);
        println!();
        ctx.log.success(
            &format!(
                "Auto-discovery complete! Added {} new generators",
                added.to_string().green().bold()
            ),
            0,
        );
    } else {
        println!(
            " {} {}",
            "ℹ".blue().bold(),
            "All discovered apps are already active.".dimmed()
        );
    }
    Ok(())
}

/// Render generators list
fn render_list(
    gen_type: Option<GeneratorType>,
    status_filter: Option<StateFilter>,
    ctx: &IrisContext,
) -> anyhow::Result<()> {
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

    if !ctx.log.quiet {
        ui::render_list_header(gen_type.is_some() || status_filter.is_some());
    }

    for g in filtered {
        let is_enabled = ctx.state.is_enabled(g.name());
        let is_installed = g.is_installed();

        if ctx.log.quiet {
            ui::render_quiet_row(g, is_enabled);
        } else {
            ui::render_full_row(g, is_enabled, is_installed);
        }
    }

    if !ctx.log.quiet {
        ui::render_list_footer(total, enabled_count, status_filter.is_none());
    } else if total > 0 {
        println!("\nTotal: {} (Enabled: {})", total, enabled_count);
    }

    Ok(())
}
