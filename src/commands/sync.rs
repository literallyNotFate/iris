use crate::{
    commands::apply_theme,
    core::{IrisContext, ThemeOrchestrator},
    models::Theme,
};

/// Handle application sync command
pub fn exec(force: bool, ctx: &mut IrisContext) -> anyhow::Result<()> {
    use colored::*;
    println!(
        "\n{}  {}",
        "󰓦".cyan().bold(),
        "Synchronizing system state...".bold()
    );

    let orchestrator: ThemeOrchestrator = ThemeOrchestrator::new(&ctx.paths, &ctx.log);
    let (nvim_theme, is_synced) = orchestrator.get_sync_status(&ctx.state);
    let is_dirty: bool = ctx.is_any_config_broken();

    if is_synced && !is_dirty && !force {
        ctx.log.success("Everything is already in sync\n");
        return Ok(());
    }

    if is_dirty && is_synced && !force {
        ctx.log.warn("Found broken configs, restoring...\n");
    }

    let theme_obj: Theme = orchestrator.load_theme(&nvim_theme, force, true, &ctx.state)?;
    apply_theme(&theme_obj, ctx)?;

    ctx.log.success("All apps are now in sync!\n");
    Ok(())
}
