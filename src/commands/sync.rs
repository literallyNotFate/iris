use crate::{commands::apply_theme, core::IrisContext, models::Palette};
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

    println!();
    if theme.to_lowercase() == ctx.state.current_theme.to_lowercase() {
        ctx.log.success("Everything is already in sync", 0);
        return Ok(());
    }

    apply_theme(&theme, ctx)?;
    ctx.log.success("All apps are now in sync!", 0);
    Ok(())
}
