use crate::{core::IrisContext, models::Palette, modules, utils::Status};
use anyhow::{Context, Result};
use colored::*;

/// Handle application switch command
pub fn exec(name: Option<String>, ctx: &mut IrisContext) -> Result<()> {
    let (theme, is_manual) = match name {
        Some(n) => (n, true),
        None => {
            let task = Status::step("Detecting theme from Neovim...", 0);
            let t = Palette::current()?;
            task.done(Some("Name not specified, using current Neovim theme"));
            (t, false)
        }
    };

    if is_manual && !Palette::exists(&theme) {
        Status::error(
            &format!("Theme '{}' not found in Neovim.", theme.red().bold()),
            0,
        );
        println!(
            "  {} Run `:colorscheme <Tab>` in Neovim to see themes.",
            "Tip:".blue()
        );
        return Ok(());
    }

    println!(
        "\n{}\n",
        format!("Switching to {}...", theme).bold().yellow()
    );

    let switch_task = Status::step(&format!("Applying {} palette...", theme.cyan()), 0);
    let palette = Palette::fetch(&theme)
        .with_context(|| format!("Failed to fetch colors for '{}'", theme))?;

    modules::apply_all(&palette, ctx)?;

    let state_task = Status::step("Updating local state...", 1);
    ctx.update(&theme)?;
    state_task.done(Some("Local state updated!"));

    println!();
    switch_task.done(Some(&format!(
        "Theme {} applied to all apps.",
        theme.cyan()
    )));

    Ok(())
}
