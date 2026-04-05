use crate::{cli::Commands, core::IrisContext};
use anyhow::{Context, Result};

pub mod generators;
pub mod setup;
pub mod status;
pub mod switch;
pub mod sync;

/// Handles all commands
pub fn handle(command: Commands, ctx: &mut IrisContext) -> Result<()> {
    match command {
        Commands::Init => setup::exec(ctx)?,
        Commands::Switch { name } => switch::exec(name, ctx)?,
        Commands::Sync => sync::exec(ctx)?,
        Commands::Status => status::exec(ctx)?,
        Commands::Gen { action } => generators::exec(action, ctx)?,
    }

    Ok(())
}

use crate::models::Palette;
use colored::Colorize;

/// Helper function to apply theme
pub(crate) fn apply_theme(theme: &str, ctx: &mut IrisContext) -> Result<()> {
    if !ctx.log.quiet {
        println!(
            "\n {} {}",
            "󰚔".green().bold(),
            format!("Applying {}...", theme).bold()
        );
        println!();
    }

    let palette = {
        let mut t = ctx.log.step(&format!("Fetching colors: {}", theme), 1);
        let p = Palette::fetch(theme, &ctx.log)
            .with_context(|| format!("Failed to fetch colors for '{}'", theme))?;
        t.done(true);
        p
    };

    ctx.registry.apply_all(&palette, ctx)?;

    {
        let mut state_task = ctx.log.step("Updating local state...", 1);
        ctx.update(theme)?;
        state_task.done(true);
    }

    Ok(())
}
