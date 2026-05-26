use crate::{cli::Commands, core::IrisContext, models::Theme};

pub mod apply;
pub mod cache;
pub mod config;
pub mod generators;
pub mod health;
pub mod preview;
pub mod select;
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
        Commands::Select => select::exec(ctx)?,
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
pub(crate) fn apply_theme(theme: &Theme, ctx: &mut IrisContext) -> anyhow::Result<()> {
    use colored::*;

    let registry = &ctx.registry;
    let state = &ctx.state;
    let paths = &ctx.paths;
    let templater = &ctx.templater;
    let log = ctx.log.clone();

    if !log.quiet {
        println!(
            "{}  {}\n",
            "󰚔".green().bold(),
            format!("Applying {}...", theme.name).bold()
        );
    }

    registry.apply_all(theme, state, paths, templater, &log)?;
    log.action("Updated local state", move || {
        ctx.update(&theme.name.clone())
    })?;

    println!("\n");
    Ok(())
}
