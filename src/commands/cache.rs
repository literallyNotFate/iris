use crate::{cli::CacheAction, core::IrisContext, utils};
use colored::Colorize;
use dialoguer::{Confirm, theme::ColorfulTheme};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

/// Handle application cache command and its subcommands
pub fn exec(action: CacheAction, ctx: &mut IrisContext) -> anyhow::Result<()> {
    match action {
        CacheAction::Clear { all, generator } => {
            let generator_name: Option<&str> = generator.as_deref();

            println!();
            if all {
                println!(
                    "{}  {} {}",
                    "".yellow().bold(),
                    "DANGER:".yellow().bold(),
                    "Nuclear option. Purging everything.".bold()
                );
            } else if let Some(name) = generator_name {
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

                let bin: &str = if which::which("bat").is_ok() {
                    "bat"
                } else {
                    "batcat"
                };

                let _ = Command::new(bin).args(["cache", "--clear"]).status();
                step.done(true);
            } else if let Some(name) = generator_name {
                let g = ctx.registry.get(name).ok_or_else(|| {
                    anyhow::anyhow!(
                        "Unknown generator: `{}`. Use `{}` to see available.",
                        name.bold().green(),
                        "iris gen list".italic().cyan()
                    )
                })?;

                let mut step = ctx.log.step(
                    &format!("Cleaning `{}` cache...", g.name().cyan().bold()),
                    1,
                );
                g.clear(ctx)?;
                step.done(true);
            } else {
                let mut step = ctx.log.step("Cleaning generated configs...", 1);
                ctx.paths.clean_gen()?;
                step.done(true);
            }
        }

        CacheAction::Remove { theme } => {
            let theme_lower: String = theme.to_lowercase();
            let path: PathBuf = ctx.paths.palettes.join(format!("{}.json", theme_lower));

            if ctx.state.current_theme == theme_lower {
                anyhow::bail!(
                    "Cannot remove active theme `{}`.",
                    utils::capitalize(&theme).cyan().bold()
                );
            }
            if ctx.state.fallback_theme == theme_lower {
                anyhow::bail!(
                    "Cannot remove fallback theme `{}`.",
                    utils::capitalize(&theme).magenta().bold()
                );
            }

            match fs::remove_file(&path) {
                Ok(_) => {
                    let mut s = ctx
                        .log
                        .step(&format!("Removing {} from cache", theme.yellow()), 1);
                    s.done(true);
                    println!(
                        "\n {}  Theme `{}` has been purged from cache.",
                        "󰆴".red().bold(),
                        utils::capitalize(&theme).cyan()
                    );
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    anyhow::bail!(
                        "Theme `{}` not found in cache. Maybe it was already removed?",
                        utils::capitalize(&theme).yellow()
                    );
                }
                Err(e) => {
                    return Err(anyhow::Error::new(e).context(format!(
                        "Failed to delete theme file at {}",
                        utils::pretty_path(&path)
                    )));
                }
            }
        }

        CacheAction::List => {
            let files = fs::read_dir(&ctx.paths.palettes)?;
            println!("\n{}  {}\n", "󰋽".cyan().bold(), "Cached Palettes:".bold());

            for entry in files.flatten() {
                let name: String = entry.file_name().to_string_lossy().replace(".json", "");
                let metadata = entry.metadata()?;
                let bytes: u64 = metadata.len();

                let size_str: String = if bytes < 1024 {
                    format!("{} B ", bytes)
                } else {
                    format!("{:.1} KB", bytes as f64 / 1024.0)
                };

                let mut line: String = format!(
                    "  • {:<15} {:>8}",
                    utils::capitalize(&name),
                    size_str.dimmed()
                );

                if name == ctx.state.current_theme {
                    line = format!("{} {}", line, "󰄬 (active)".green());
                } else if name == ctx.state.fallback_theme {
                    line = format!("{} {}", line, "󰁯 (fallback)".blue());
                }

                println!("{}", line);
            }
        }

        CacheAction::Info => {
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
            let palette_size = get_directory_size(&ctx.paths.palettes)?;
            let gen_size = get_directory_size(&ctx.paths.generators)?;

            println!(
                "    {:<12} {} files ({:.1} KB)",
                "Palettes",
                palette_count,
                palette_size as f64 / 1024.0
            );
            println!("    {:<12} {:.1} KB", "Configs", gen_size as f64 / 1024.0);

            println!("\n  {}", "Current State:".red());
            println!(
                "    {:<12} {}",
                "Theme",
                ctx.state.current_theme.green().bold()
            );
            println!("    {:<12} {}", "Fallback", ctx.state.fallback_theme.blue());
            println!(
                "    {:<12} {}",
                "Strategy",
                format!("{:?}", ctx.state.nvim).yellow()
            );
        }
    }

    println!(
        "\n{}  {}",
        "".green().bold(),
        "Operation complete.".green().bold()
    );
    Ok(())
}

/// Helper function to get directory size
fn get_directory_size(path: &Path) -> anyhow::Result<u64> {
    let mut size = 0;
    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            if metadata.is_dir() {
                size += get_directory_size(&entry.path())?;
            } else {
                size += metadata.len();
            }
        }
    }
    Ok(size)
}
