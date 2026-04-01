use crate::{
    commands::apply_theme,
    core::IrisContext,
    models::Palette,
    utils::{self, Status},
};
use colored::Colorize;

/// Handle application switch command
pub fn exec(name: String, ctx: &mut IrisContext) -> anyhow::Result<()> {
    println!(
        "\n {}  {}",
        "󰚔".yellow().bold(),
        "Manual theme switch".bold()
    );

    if !Palette::exists(&name) {
        println!();
        Status::error(
            &format!(
                "Theme '{}' not found in Neovim.",
                utils::capitalize(&name).red().bold()
            ),
            0,
        );
        println!(
            "  {}  Run `:colorscheme <Tab>` in Neovim to see all available themes.",
            "󰋗 Tip:".blue()
        );
        return Ok(());
    }

    apply_theme(&name, ctx)
}
