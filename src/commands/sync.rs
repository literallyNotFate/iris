use crate::{commands::apply_theme, core::IrisContext};
use colored::Colorize;

/// Handle application sync command
pub fn exec(force: bool, ctx: &mut IrisContext) -> anyhow::Result<()> {
    println!(
        "\n {}  {}",
        "󰓦".cyan().bold(),
        "Synchronizing system state...".bold()
    );

    let (nvim_theme, is_synced) = ctx.get_sync_status();
    let is_dirty: bool = ctx.is_any_config_broken();

    if is_synced && !is_dirty && !force {
        println!();
        ctx.log.success("Everything is already in sync", 0);
        return Ok(());
    }

    println!();
    if is_dirty && is_synced {
        ctx.log.warn("Found broken configs, restoring...", 0);
    } else {
        ctx.log
            .step(&format!("Syncing theme: {} 󰁯 Neovim", nvim_theme.cyan()), 1);
    }

    apply_theme(&nvim_theme, force, ctx)?;
    ctx.log.success("All apps are now in sync!", 0);
    Ok(())
}
