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
    println!();

    if !Palette::exists(&name, &ctx.log) {
        println!();
        ctx.log.error(
            &format!(
                "Theme '{}' not found in Neovim.",
                utils::capitalize(&name).red().bold()
            ),
            0,
        );

        if !ctx.log.quiet {
            println!(
                "{} Run `:colorscheme <Tab>` in Neovim to see all available themes.",
                "󰋗 Tip:".blue()
            );
        }
        return Ok(());
    }

    apply_theme(&name, ctx)
}
