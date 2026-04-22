use crate::{cli::ConfigAction, core::IrisContext, models::NvimStrategy};
use colored::*;

/// Handle application config command
pub fn exec(action: ConfigAction, ctx: &mut IrisContext) -> anyhow::Result<()> {
    match action {
        ConfigAction::Nvim { strategy, detect } => {
            render_config_header("Neovim Configuration", "⚙");

            let selected: NvimStrategy = NvimStrategy::choose(strategy, detect, &ctx.log)?;
            selected.validate()?;

            render_nvim_status(&selected);
            ctx.state.nvim = selected;
        }

        ConfigAction::Fallback { name } => {
            render_config_header("Fallback Configuration", "⚙");
            let theme: String = name.to_lowercase();
            ctx.validate_theme_exists(&theme)?;

            println!(
                "\n{}  {} set to {}",
                "󰁯".bright_magenta().bold(),
                "Fallback theme".bold(),
                theme.cyan().bold()
            );

            ctx.state.fallback_theme = theme;
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

/// Helper function to render header for config
fn render_config_header(title: &str, icon: &str) {
    println!("\n{}  {}", icon.bright_yellow().bold(), title.bold());
}

/// Helper function to render nvim status w/strategy
fn render_nvim_status(strategy: &NvimStrategy) {
    let count = strategy.count_plugins();
    if count > 0 {
        println!(
            "      {}  Status: {} plugins found",
            "󰄬".green().bold(),
            count.to_string().cyan().bold()
        );
    }
}
