use crate::{
    commands::apply_theme,
    core::IrisContext,
    models::{NvimStrategy, Palette},
    utils::{self},
};
use colored::Colorize;

/// Handle application switch command
pub fn exec(name: String, force: bool, ctx: &mut IrisContext) -> anyhow::Result<()> {
    let original_name: String = name.clone();
    let mut target_name: String = name.to_lowercase();
    let mut is_fallback: bool = false;

    if !Palette::exists(&target_name, ctx) {
        anyhow::bail!(
            "Theme `{}` not found.\n{}  Run `:colorscheme <Tab>` in Neovim to see all available themes.",
            utils::capitalize(&target_name).yellow().bold(),
            "󰋗".blue()
        );
    }

    println!();
    if matches!(ctx.state.nvim, NvimStrategy::Default) {
        let builtins: Vec<String> = NvimStrategy::get_builtin_themes();

        if !builtins.contains(&target_name) {
            if force
                && ctx
                    .paths
                    .palettes
                    .join(format!("{}.json", target_name))
                    .exists()
            {
                println!(
                    " {}  {} Using cached `{}` despite Built-in strategy due to {} flag.",
                    "󰚔".cyan(),
                    "Force:".cyan().bold(),
                    target_name.white().bold(),
                    "--force".yellow()
                );
            } else {
                is_fallback = true;
                target_name = builtins
                    .iter()
                    .find(|&&ref s| s == "retrobox")
                    .cloned()
                    .unwrap_or_else(|| "default".into());
            }
        }
    }

    if is_fallback {
        println!(
            " {}  {} `{}` is a plugin-based theme, but you are using the Built-in strategy.",
            "".yellow(),
            "Notice:".yellow().bold(),
            original_name.white().bold()
        );
        println!(
            " {}  Falling back to stable theme: {}",
            "󰋽".blue(),
            utils::capitalize(&target_name).green().bold()
        );
    } else {
        println!(
            "\n {}  Palette for {} loaded!",
            "󰄬".green().bold(),
            utils::capitalize(&target_name).yellow().bold()
        );
    }

    apply_theme(&target_name, force, ctx)
}
