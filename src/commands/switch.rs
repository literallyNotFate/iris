use crate::{
    commands::apply_theme,
    core::IrisContext,
    models::{NvimStrategy, Palette},
};
use colored::Colorize;

/// Handle application switch command
pub fn exec(
    name: String,
    force: bool,
    fallback: bool,
    ctx: &mut IrisContext,
) -> anyhow::Result<()> {
    let mut target_name: String = name.to_lowercase();
    let mut is_fallback_applied: bool = false;

    if !Palette::exists(&target_name, ctx) {
        if force {
            println!(
                "\n {} Theme not found in known sources. {} will attempt a deep fetch via Neovim.",
                "󰚔 ".cyan(),
                "--force".yellow()
            );
        } else if fallback {
            target_name = ctx.state.fallback_theme.clone();
            is_fallback_applied = true;
        } else {
            anyhow::bail!(
                "Theme `{}` not found. Use `--fallback` or `--force`.",
                name.yellow().bold()
            );
        }
    }

    if matches!(ctx.state.nvim, NvimStrategy::Default) {
        let builtins: Vec<String> = NvimStrategy::get_builtin_themes();

        let has_cache: bool = ctx
            .paths
            .palettes
            .join(format!("{}.json", target_name))
            .exists();

        if !builtins.contains(&target_name) && !has_cache && !force {
            if fallback {
                target_name = ctx.state.fallback_theme.clone();
                is_fallback_applied = true;
            } else {
                anyhow::bail!(
                    "Theme `{}` is external and not cached. Strategy is set to Built-in.",
                    target_name.yellow()
                );
            }
        }
    }

    if is_fallback_applied {
        if !Palette::exists(&target_name, ctx) {
            anyhow::bail!(
                "Primary theme and fallback theme `{}` are both unavailable.",
                target_name.red().bold()
            );
        }

        println!(
            "\n {}  Theme `{}` unavailable or restricted. Using fallback: {}",
            "󰁯".blue(),
            name.dimmed(),
            target_name.green().bold()
        );
    }

    apply_theme(&target_name, force, ctx)
}
