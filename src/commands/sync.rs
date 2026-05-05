use crate::{commands::apply_theme, core::IrisContext, models::Palette};

/// Handle application sync command
pub fn exec(force: bool, ctx: &mut IrisContext) -> anyhow::Result<()> {
    use colored::*;
    println!(
        "\n{}  {}",
        "󰓦".cyan().bold(),
        "Synchronizing system state...".bold()
    );

    let (nvim_theme, is_synced) = ctx.get_sync_status();
    let is_dirty: bool = ctx.is_any_config_broken();

    if is_synced && !is_dirty && !force {
        ctx.log.success("Everything is already in sync\n");
        return Ok(());
    }

    if is_dirty && is_synced && !force {
        ctx.log.warn("Found broken configs, restoring...\n");
    }

    let palette = Palette::fetch(&nvim_theme, force, true, &ctx.paths, &ctx.state, &ctx.log)?;
    apply_theme(&palette, ctx)?;

    ctx.log.success("All apps are now in sync!\n");
    Ok(())
}
