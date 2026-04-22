use crate::{cli::CacheAction, core::IrisContext, utils};
use colored::Colorize;
use dialoguer::{Confirm, theme::ColorfulTheme};
use std::fs;

/// Handle application cache command and its subcommands
pub fn exec(action: CacheAction, ctx: &IrisContext) -> anyhow::Result<()> {
    match action {
        CacheAction::Clear { all, generator } => handle_clear(all, generator, ctx)?,
        CacheAction::Remove { theme } => handle_remove(&theme, ctx)?,
        CacheAction::List => render_list(ctx)?,
        CacheAction::Info => render_info(ctx)?,
    }

    println!(
        "\n{}  {}",
        "".green().bold(),
        "Operation complete.".green().bold()
    );
    Ok(())
}

/// Cache clear (all, for generator or config)
fn handle_clear(all: bool, gen_name: Option<String>, ctx: &IrisContext) -> anyhow::Result<()> {
    println!();
    if all {
        println!(
            "{}  {} {}",
            "".yellow().bold(),
            "DANGER:".yellow().bold(),
            "Nuclear option. Purging everything.".bold()
        );
    } else if let Some(ref name) = gen_name {
        println!(
            "{}  Cleaning cache for: {}",
            "󰋽".cyan().bold(),
            name.green().bold()
        );
    }

    if !Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Do you want to proceed?")
        .default(false)
        .interact()?
    {
        println!("\n{}  {}", "󰋽".dimmed(), "Canceled.".dimmed());
        return Ok(());
    }

    if all {
        let mut step = ctx.log.step("Purging iris cache...", 1);
        ctx.paths.purge_all()?;
        utils::external::clear_bat_cache();
        step.done(true);
    } else if let Some(name) = gen_name {
        let g = ctx.resolve_generator(&name)?;
        let mut step = ctx.log.step(
            &format!("Cleaning `{}` cache...", g.name().cyan().bold()),
            1,
        );
        g.clear(&ctx.paths)?;
        step.done(true);
    } else {
        let mut step = ctx.log.step("Cleaning generated configs...", 1);
        ctx.paths.clean_gen()?;
        step.done(true);
    }
    Ok(())
}

/// Removes requested palette from the cache
fn handle_remove(theme: &str, ctx: &IrisContext) -> anyhow::Result<()> {
    let theme_lower = theme.to_lowercase();
    let path = ctx.paths.palettes.join(format!("{}.json", theme_lower));

    if ctx.state.current_theme == theme_lower {
        anyhow::bail!(
            "Cannot remove active theme `{}`.",
            utils::capitalize(theme).cyan().bold()
        );
    }
    if ctx.state.fallback_theme == theme_lower {
        anyhow::bail!(
            "Cannot remove fallback theme `{}`.",
            utils::capitalize(theme).magenta().bold()
        );
    }

    fs::remove_file(&path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            anyhow::anyhow!("Theme `{}` not found in cache.", theme.yellow())
        } else {
            anyhow::Error::new(e).context("Failed to delete theme file")
        }
    })?;

    let mut s = ctx
        .log
        .step(&format!("Removing {} from cache", theme.yellow()), 1);
    s.done(true);

    println!(
        "\n {}  Theme `{}` has been purged.",
        "󰆴".red().bold(),
        utils::capitalize(theme).cyan()
    );
    Ok(())
}

/// List of cached palettes
fn render_list(ctx: &IrisContext) -> anyhow::Result<()> {
    println!("\n{}  {}\n", "󰋽".cyan().bold(), "Cached Palettes:".bold());

    for entry in fs::read_dir(&ctx.paths.palettes)?.flatten() {
        let name = entry.file_name().to_string_lossy().replace(".json", "");
        let size = entry.metadata()?.len();

        let mut line = format!(
            "  • {:<15} {:>8}",
            utils::capitalize(&name),
            utils::format_size(size).dimmed()
        );

        if name == ctx.state.current_theme {
            line.push_str(&format!(" {}", "󰄬 (active)".green()));
        } else if name == ctx.state.fallback_theme {
            line.push_str(&format!(" {}", "󰁯 (fallback)".blue()));
        }
        println!("{}", line);
    }
    Ok(())
}

/// Handling cache info (status)
fn render_info(ctx: &IrisContext) -> anyhow::Result<()> {
    println!(
        "\n{}  {}",
        "󰋽".cyan().bold(),
        "Iris Cache Information:".bold()
    );

    println!("\n  {}", "Locations:".magenta());
    let locations = [
        ("Root", &ctx.paths.cache),
        ("Palettes", &ctx.paths.palettes),
        ("Generators", &ctx.paths.generators),
    ];

    for (label, path) in locations {
        println!(
            "    {:<12} {}",
            label.white(),
            utils::pretty_path(path).cyan()
        );
    }

    println!("\n  {}", "Usage Stats:".yellow());
    let palette_count = fs::read_dir(&ctx.paths.palettes)?.count();
    let p_size = ctx.paths.get_size(&ctx.paths.palettes);
    let g_size = ctx.paths.get_size(&ctx.paths.generators);

    println!(
        "    {:<12} {} files ({})",
        "Palettes",
        palette_count,
        utils::format_size(p_size)
    );
    println!("    {:<12} {}", "Configs", utils::format_size(g_size));

    println!("\n  {}", "Current State:".red());
    println!(
        "    {:<12} {}",
        "Theme",
        ctx.state.current_theme.green().bold()
    );
    println!("    {:<12} {}", "Fallback", ctx.state.fallback_theme.blue());
    println!("    {:<12} {}", "Strategy", ctx.state.nvim);

    Ok(())
}
