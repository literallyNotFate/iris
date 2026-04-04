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

use crate::{
    models::Palette,
    utils::{self, Status},
};
use colored::Colorize;

/// Helper function to apply theme
pub(crate) fn apply_theme(theme: &str, ctx: &mut IrisContext) -> Result<()> {
    println!(
        "\n {} {}",
        "󰚔".green().bold(),
        format!("Applying {}...", theme.bold()).yellow()
    );
    println!("{}", " ─────────────────────────────────────────".dimmed());

    let switch_task = Status::step("Applying palette to generators", 0);
    let palette =
        Palette::fetch(theme).with_context(|| format!("Failed to fetch colors for '{}'", theme))?;

    ctx.registry.apply_all(&palette, ctx)?;
    switch_task.done(Some(&format!(
        "{} applied to all active apps",
        utils::capitalize(theme)
    )));

    let state_task = Status::step("Updating local state", 0);
    ctx.update(theme)?;
    state_task.done(Some("state.json updated"));

    println!(
        "\n {} {}",
        "󰄬".green().bold(),
        "All systems updated successfully!".bold()
    );

    Ok(())
}
