use crate::{
    cli::Commands,
    core::{IrisContext, IrisSetup},
    log::LoggingVerbosity,
    models::Theme,
};

pub mod apply;
pub mod cache;
pub mod config;
pub mod current;
pub mod diff;
pub mod generators;
pub mod health;
pub mod preview;
pub mod select;
pub mod status;
pub mod switch;
pub mod sync;
pub mod toggle;
pub mod watch;

/// Main entry point for command execution.
/// Routes CLI commands to their respective logic modules
pub fn handle(command: Commands, ctx: &mut IrisContext) -> anyhow::Result<()> {
    match command {
        Commands::Init => {
            if ctx.log.verbosity == LoggingVerbosity::Silent {
                IrisSetup::emit_zsh_hook(ctx)?;
                return Ok(());
            }

            IrisSetup::run(ctx)?;
        }
        Commands::Switch(args) => switch::exec(args, ctx)?,
        Commands::Sync { force } => sync::exec(force, ctx)?,
        Commands::Apply(args) => apply::exec(args, ctx)?,
        Commands::Select => select::exec(ctx)?,
        Commands::Status => status::exec(ctx)?,
        Commands::Preview { theme } => preview::exec(theme, ctx)?,
        Commands::Watch { interval } => watch::exec(interval, ctx)?,
        Commands::Health { fix } => health::exec(fix, ctx)?,
        Commands::Gen { action } => generators::exec(action, ctx)?,
        Commands::Diff { generator } => diff::exec(generator, ctx)?,
        Commands::Cache { action } => cache::exec(action, ctx)?,
        Commands::Config { action } => config::exec(action, ctx)?,
        Commands::Toggle => toggle::exec(ctx)?,
        Commands::Current => current::exec(ctx)?,
        Commands::CompleteList => {
            if let Ok(themes) = ctx.get_available_themes() {
                for theme in themes {
                    println!("{}", theme);
                }
            }
            return Ok(());
        }
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

    if log.is_detailed() {
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

    println!();
    Ok(())
}
