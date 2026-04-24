use crate::{cli::Commands, core::IrisContext, models::Palette, utils};
use colored::Colorize;

pub mod apply;
pub mod cache;
pub mod config;
pub mod generators;
pub mod health;
pub mod preview;
pub mod setup;
pub mod status;
pub mod switch;
pub mod sync;
pub mod watch;

/// Main entry point for command execution
/// Routes CLI commands to their respective logic modules
pub fn handle(command: Commands, ctx: &mut IrisContext) -> anyhow::Result<()> {
    match command {
        Commands::Init => setup::exec(ctx)?,
        Commands::Switch(args) => switch::exec(args, ctx)?,
        Commands::Sync { force } => sync::exec(force, ctx)?,
        Commands::Apply(args) => apply::exec(args, ctx)?,
        Commands::Status => status::exec(ctx)?,
        Commands::Preview { theme } => preview::exec(theme, ctx)?,
        Commands::Watch { interval } => watch::exec(interval, ctx)?,
        Commands::Health { fix } => health::exec(fix, ctx)?,
        Commands::Gen { action } => generators::exec(action, ctx)?,
        Commands::Cache { action } => cache::exec(action, ctx)?,
        Commands::Config { action } => config::exec(action, ctx)?,
    }

    Ok(())
}

/// Helper function to apply theme
pub(crate) fn apply_theme(theme: &str, force: bool, ctx: &mut IrisContext) -> anyhow::Result<()> {
    if !ctx.log.quiet {
        println!(
            "\n {}  {}",
            "󰚔".green().bold(),
            format!("Applying {}...", utils::capitalize(theme)).bold()
        );
        println!();
    }

    let palette = {
        let mut t = ctx.log.step(&format!("Fetching colors: {}", theme), 1);
        let p = Palette::fetch(theme, force, &ctx.paths, &ctx.state, &ctx.log)?;
        t.done(true);
        p
    };

    ctx.registry.apply_all(&palette, ctx)?;

    {
        let mut state_task = ctx.log.step("Updating local state", 1);
        ctx.update(theme)?;
        state_task.done(true);
    }

    Ok(())
}
