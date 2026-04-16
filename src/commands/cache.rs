use crate::{cli::CacheAction, core::IrisContext, utils};
use colored::Colorize;
use dialoguer::{Confirm, theme::ColorfulTheme};
use std::process::Command;

/// Handle application cache command and its subcommands
pub fn exec(action: CacheAction, ctx: &mut IrisContext) -> anyhow::Result<()> {
    let (is_all, generator_name) = match &action {
        CacheAction::Clear { all, generator } => (*all, generator.as_deref()),
    };

    let target_gen = if let Some(name) = generator_name {
        let g = ctx.registry.get(name).ok_or_else(|| {
            anyhow::anyhow!(
                "Unknown generator: `{}`. Use `{}` to see available.",
                name.bold().green(),
                "iris gen list".italic().cyan()
            )
        })?;
        Some(g)
    } else {
        None
    };

    println!();
    if is_all {
        println!(
            "{}  {} {}",
            "".yellow().bold(),
            "DANGER:".yellow().bold(),
            "This will permanently delete all cached themes and data.".bold()
        );
        println!(
            "{}  Target: {}",
            "󰋽".cyan().bold(),
            utils::pretty_path(&ctx.paths.cache).italic().cyan()
        );
    } else if let Some(name) = generator_name {
        println!(
            "{}  {} Cleaning cache for generator: {}",
            "󰋽".cyan().bold(),
            "INFO:".cyan().bold(),
            name.green().bold()
        );
    } else {
        println!(
            "{}  {} Cleaning all generated config files.",
            "󰋽".cyan().bold(),
            "INFO:".cyan().bold()
        );
    }

    let confirmation = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Do you want to proceed?")
        .default(false)
        .interact()?;

    if !confirmation {
        println!("\n{}  {}", "󰋽".dimmed(), "Canceled.".dimmed());
        return Ok(());
    }
    println!();

    match action {
        CacheAction::Clear { all: true, .. } => {
            let mut step = ctx.log.step("Purging iris cache...", 1);
            ctx.paths.purge_all()?;

            let bin: &str = if which::which("bat").is_ok() {
                "bat"
            } else {
                "batcat"
            };
            let _ = Command::new(bin).args(["cache", "--clear"]).status();

            step.done(true);
        }
        CacheAction::Clear { all: false, .. } => {
            if let Some(g) = target_gen {
                if !g.is_installed() {
                    ctx.log.warn(
                        &format!(
                            "Generator '{}' is recognized, but the app is not found in the system",
                            g.name().bold()
                        ),
                        1,
                    );
                }

                let mut step = ctx
                    .log
                    .step(&format!("Cleaning {} cache...", g.name().cyan().bold()), 1);
                g.clear(ctx)?;
                step.done(true);
            } else {
                let mut step = ctx.log.step(
                    &format!(
                        "Cleaning {} directory...",
                        utils::pretty_path(&ctx.paths.generators)
                    ),
                    1,
                );
                ctx.paths.clean_gen()?;
                step.done(true);
            }
        }
    }

    println!(
        "\n{}  {}",
        "".green().bold(),
        "Success! Cache is now clean.".green().bold()
    );

    Ok(())
}
