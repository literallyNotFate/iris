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
        ctx.log.info("Canceled.");
        println!();
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

/// Removes requested theme from the cache along with the config files for generator
fn handle_remove(theme: &str, ctx: &IrisContext) -> Result<()> {
    let theme_lower: String = theme.to_lowercase();
    let path: PathBuf = ctx.paths.cached_theme(theme);

    if ctx.state.current_theme == theme_lower {
        anyhow::bail!("Cannot remove active theme `{}`.", theme.cyan().bold());
    }

    if ctx.state.fallback_theme == theme_lower {
        anyhow::bail!("Cannot remove fallback theme `{}`.", theme.magenta().bold());
    }

    for generator in ctx.registry.all() {
        let _ = generator.remove_theme(&ctx.paths, &theme_lower);
    }

    if !path.exists() {
        anyhow::bail!("Theme not found in cache.");
    }

    println!();
    ctx.log
        .action(&format!("Removed `{}` from cache", theme.yellow()), || {
            fs::remove_file(&path).context("Failed to delete theme file")
        })?;

    println!();
    Ok(())
}

/// List of cached themes
fn render_list(ctx: &IrisContext) -> Result<()> {
    let themes: Vec<String> = ctx.paths.get_cached_themes()?;
    println!();

    if ctx.log.is_detailed() {
        println!("{}  {}\n", "󰋽".cyan().bold(), "Cached Themes:".bold());
        for name in themes {
            let file_path = ctx.paths.cached_theme(&name);
            let size = fs::metadata(&file_path)?.len();
            let display_name = utils::capitalize(&name);

            let status = if name == ctx.state.current_theme {
                "✓  (active)".green()
            } else if name == ctx.state.fallback_theme {
                "󰁯  (fallback)".blue()
            } else {
                "".into()
            };

            println!(
                "  • {:<15} {:>8}  {}",
                display_name,
                utils::format_size(size).dimmed(),
                status
            );
        }
    } else {
        let list = themes
            .iter()
            .map(|name| {
                if name == &ctx.state.current_theme {
                    format!("{}*", utils::capitalize(name).green().bold())
                } else if name == &ctx.state.fallback_theme {
                    format!("{}!", utils::capitalize(name).blue())
                } else {
                    utils::capitalize(name)
                }
            })
            .collect::<Vec<_>>()
            .join(", ");

        println!("{}  Cached themes: {}", "󰋽".cyan().bold(), list);
    }

    println!();
    Ok(())
}

/// Handling cache info (status)
fn render_info(ctx: &IrisContext) -> Result<()> {
    println!();

    let palette_count: usize = fs::read_dir(&ctx.paths.themes)?.count();
    let p_size: u64 = ctx.paths.get_size(&ctx.paths.themes);
    let g_size: u64 = ctx.paths.get_size(&ctx.paths.generators);

    if ctx.log.is_detailed() {
        println!(
            "{}  {}",
            "󰋽".cyan().bold(),
            "Iris Cache Information:".bold()
        );
        println!("\n  {}", "Locations:".magenta());
        let locations = [
            ("Root", &ctx.paths.cache),
            ("Palettes", &ctx.paths.themes),
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
            utils::capitalize(&ctx.state.current_theme).green().bold()
        );
        println!(
            "    {:<12} {}",
            "Fallback",
            utils::capitalize(&ctx.state.fallback_theme).blue()
        );
        println!("    {:<12} {}", "Manager", ctx.state.manager);
    } else {
        println!(
            "{}  Cache: {} palettes ({}) | Configs: {} | Active: {}",
            "󰋽".cyan().bold(),
            palette_count,
            utils::format_size(p_size).yellow(),
            utils::format_size(g_size).yellow(),
            utils::capitalize(&ctx.state.current_theme).green().bold()
        );
    }

    println!();
    Ok(())
}
