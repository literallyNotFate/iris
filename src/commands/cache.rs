use crate::{cli::CacheAction, core::IrisContext, utils};
use colored::Colorize;
use dialoguer::{Confirm, theme::ColorfulTheme};

/// Handle application cache command and its subcommands
pub fn exec(action: CacheAction, ctx: &mut IrisContext) -> anyhow::Result<()> {
    let is_all: bool = match action {
        CacheAction::Clean { all } => all,
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
        println!();
    } else {
        println!(
            "{}  {} {}",
            "󰋽".cyan().bold(),
            "INFO:".cyan().bold(),
            "Cleaning generated config files only."
        );
    }

    let confirmation: bool = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Do you want to proceed?")
        .default(false)
        .interact()?;

    if !confirmation {
        println!("\n{}  {}", "󰋽".dimmed(), "Canceled.".dimmed());
        return Ok(());
    }

    println!();
    match action {
        CacheAction::Clean { all: true } => {
            let mut step = ctx.log.step("Purging iris cache...", 1);
            ctx.paths.purge_all()?;

            let bin: &str = if which::which("bat").is_ok() {
                "bat"
            } else {
                "batcat"
            };

            let _ = std::process::Command::new(bin)
                .args(["cache", "--clear"])
                .output()?;

            step.done(true);
        }
        CacheAction::Clean { all: false } => {
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

    println!(
        "\n{}  {}",
        "".green().bold(),
        "Success! Cache is now clean.".green().bold()
    );

    Ok(())
}
