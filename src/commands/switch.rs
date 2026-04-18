use crate::{
    commands::apply_theme,
    core::IrisContext,
    models::Palette,
    utils::{self},
};
use colored::Colorize;

/// Handle application switch command
pub fn exec(name: String, ctx: &mut IrisContext) -> anyhow::Result<()> {
    println!(
        "\n {}  {}",
        "󰚔".yellow().bold(),
        "Manual theme switch".bold()
    );

    if !Palette::exists(&name, ctx) {
        anyhow::bail!(
            "Theme '{}' not found in cache or Neovim.\n{}  Run `:colorscheme <Tab>` in Neovim to see all available themes.",
            utils::capitalize(&name).yellow().bold(),
            "󰋗".blue()
        );
    }

    println!(
        "\n {}  Theme {} found!",
        "󰄬".green().bold(),
        utils::capitalize(&name).yellow().bold()
    );

    apply_theme(&name, ctx)
}
