use crate::{commands::apply_theme, core::IrisContext, models::Palette, utils::Status};
use colored::Colorize;

/// Handle application sync command
pub fn exec(ctx: &mut IrisContext) -> anyhow::Result<()> {
    println!(
        "\n {}  {}",
        "󰓦".cyan().bold(),
        "Synchronizing with Neovim...".bold()
    );

    let theme: String = Palette::current()?;
    if theme.to_lowercase() == ctx.state.current_theme.to_lowercase() {
        Status::success("Everything is already in sync", 0);
        return Ok(());
    }

    apply_theme(&theme, ctx)?;

    println!(" {} {}", "󰄬".green(), "All apps are now in sync".dimmed());
    Ok(())
}
