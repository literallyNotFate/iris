use crate::{cli::CacheAction, core::IrisContext, utils};
use anyhow::{Context, Result};
use colored::Colorize;
use dialoguer::Confirm;
use std::{fs, path::PathBuf};

/// Handle application cache command and its subcommands
pub fn exec(action: CacheAction, ctx: &IrisContext) -> Result<()> {
    match action {
        CacheAction::Clear { all, generator } => handle_clear(all, generator, ctx)?,
        CacheAction::Remove { theme } => handle_remove(&theme, ctx)?,
        CacheAction::List => render_list(ctx)?,
        CacheAction::Info => render_info(ctx)?,
    }

    println!();
    Ok(())
}

/// Cache clear (all, for generator or config)
fn handle_clear(all: bool, gen_name: Option<String>, ctx: &IrisContext) -> Result<()> {
    println!();

    let generator = if let Some(ref name) = gen_name {
        let g = ctx.resolve_generator(name)?;
        Some(g)
    } else {
        None
    };

    let prompt_message = match (&generator, all) {
        (Some(g), _) => format!(
            "Do you want to clear the cache for the `{}` generator?",
            g.name().cyan().bold()
        ),
        (None, true) => format!(
            "{}: This will wipe all Iris caches and configurations. Are you sure?",
            "DANGER".bold()
        ),
        (None, false) => "Do you want to clean generated configurations?".to_string(),
    };

    if all {
        ctx.log.warn(&format!(
            "{}: Nuclear option. Purging everything.",
            "DANGER".bold()
        ));
    } else if let Some(ref g) = generator {
        ctx.log.info(&format!(
            "Cleaning cache for generator: {}",
            g.name().green().bold()
        ));
    } else {
        ctx.log.info("Cleaning generated configurations");
    }

    if !Confirm::with_theme(&utils::colors::select_theme())
        .with_prompt(prompt_message)
        .default(false)
        .interact()?
    {
        println!();
        ctx.log.info("Canceled.");
        return Ok(());
    }

    if all {
        ctx.log.action("Purged all Iris caches\n", || {
            ctx.paths.purge_all()?;
            utils::external::clear_bat_cache();
            Ok(())
        })
    } else if let Some(ref g) = generator {
        ctx.log.action(
            &format!("Cleaned `{}` cache\n", g.name().cyan().bold()),
            || g.clear(&ctx.paths),
        )
    } else {
        ctx.log.action("Cleaned generated configurations\n", || {
            ctx.paths.clean_gen()
        })
    }
}

/// Removes requested palette from the cache
fn handle_remove(theme: &str, ctx: &IrisContext) -> Result<()> {
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

    if !path.exists() {
        anyhow::bail!("Theme not found in cache");
    }

    println!();
    ctx.log.action(
        &format!("Removed `{}` from cache", utils::capitalize(theme).yellow()),
        || fs::remove_file(&path).context("Failed to delete theme file"),
    )?;
    Ok(())
}

/// List of cached palettes
fn render_list(ctx: &IrisContext) -> Result<()> {
    println!("\n{}  {}\n", "󰋽".cyan().bold(), "Cached Palettes:".bold());
    let themes: Vec<String> = ctx.paths.get_cached_themes()?;

    for name in themes {
        let file_path: PathBuf = ctx.paths.palettes.join(format!("{}.json", name));
        let size: u64 = fs::metadata(&file_path)?.len();
        let display_name: String = utils::capitalize(&name);

        let mut line = format!(
            "  • {:<15} {:>8}",
            display_name,
            utils::format_size(size).dimmed()
        );

        if name == ctx.state.current_theme {
            line.push_str(&format!(" {}", "󰄬  (active)".green()));
        } else if name == ctx.state.fallback_theme {
            line.push_str(&format!(" {}", "󰁯  (fallback)".blue()));
        }

        println!("{}", line);
    }

    Ok(())
}

/// Handling cache info (status)
fn render_info(ctx: &IrisContext) -> Result<()> {
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
