use crate::{
    cli::ConfigAction,
    core::IrisContext,
    models::{NvimStrategy, Palette},
};
use colored::*;

/// Handle application config command
pub fn exec(action: ConfigAction, ctx: &mut IrisContext) -> anyhow::Result<()> {
    match action {
        ConfigAction::Nvim { strategy, detect } => {
            println!(
                "\n{}  {}\n",
                "⚙".bright_yellow().bold(),
                "Neovim Configuration".bold()
            );

            let selected: NvimStrategy = NvimStrategy::choose(strategy, detect, ctx)?;
            selected.validate()?;

            let count: usize = selected.count_plugins();
            if count > 0 {
                println!(
                    "      {}  Status: {} plugins found",
                    "󰄬".green().bold(),
                    count.to_string().cyan().bold()
                );
            }

            ctx.state.nvim = selected;
        }

        ConfigAction::Fallback { name } => {
            let theme_lower: String = name.to_lowercase();
            if !Palette::exists(&theme_lower, ctx) {
                anyhow::bail!(
                    "Cannot set `{}` as fallback. Theme not found in cache or Neovim.",
                    name.yellow().bold()
                );
            }

            println!(
                "\n{}  {} set to {}",
                "󰁯".bright_magenta().bold(),
                "Fallback Theme".bold(),
                theme_lower.cyan().bold()
            );

            ctx.state.fallback_theme = theme_lower;
        }
    }

    println!();
    let mut s = ctx.log.step("Updating state.json", 1);
    ctx.state.save_to(&ctx.paths.state_file)?;
    s.done(true);

    println!(
        "\n{}  Configuration updated successfully.",
        "✔".green().bold()
    );

    Ok(())
}
