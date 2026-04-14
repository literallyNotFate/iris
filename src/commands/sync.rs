use crate::{
    commands::{HealthStatus, apply_theme},
    core::IrisContext,
    models::Palette,
};
use colored::Colorize;

/// Handle application sync command
pub fn exec(ctx: &mut IrisContext) -> anyhow::Result<()> {
    println!(
        "\n {}  {}",
        "󰓦".cyan().bold(),
        "Synchronizing with Neovim...".bold()
    );

    let theme = {
        let mut t = ctx.log.step("Detecting Neovim theme", 1);
        let name = Palette::current(&ctx.log)?;
        t.done(true);
        name
    };

    let is_dirty = ctx
        .registry
        .all()
        .iter()
        .any(|generator| matches!(generator.health_check(ctx), HealthStatus::Error { .. }));

    let theme_changed: bool = theme.to_lowercase() != ctx.state.current_theme.to_lowercase();

    println!();

    if !theme_changed && !is_dirty {
        ctx.log.success("Everything is already in sync", 0);
        return Ok(());
    }

    if is_dirty && !theme_changed {
        ctx.log.warn("Found broken configs, restoring...", 0);
    }

    apply_theme(&theme, ctx)?;

    ctx.log.success("All apps are now in sync!", 0);
    Ok(())
}
