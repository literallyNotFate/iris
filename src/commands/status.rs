use crate::{
    core::IrisContext,
    models::Palette,
    ui::Logger,
    utils::{self, CustomColor},
};
use colored::*;

/// Handle application status command
pub fn exec(ctx: &mut IrisContext) -> anyhow::Result<()> {
    let (nvim_theme, is_sync) = ctx.get_sync_status();
    let current: String = ctx.state.current_theme.clone();

    if ctx.log.quiet {
        return Ok(render_quiet(ctx, &current, &nvim_theme, is_sync));
    }

    println!("\n {}  {}", "󰗼".cyan().bold(), "Iris system status".bold());
    println!(
        "\n  {}  Active theme:  {}",
        "󰏘".red(),
        current.bold().blue()
    );
    println!("  {}  Plugin Manager:  {}", "⚙".magenta(), ctx.state.nvim);
    println!(
        "  {}  Config path:   {}",
        "󰉖".white(),
        utils::pretty_path(&ctx.paths.config).bright_black()
    );

    println!("\n  {}  {}", "󰒓".yellow(), "Enabled generators:".bold());
    render_generators_list(ctx);

    render_sync_block(ctx, is_sync, &current, &nvim_theme);

    let palette_result = Palette::fetch(&current, false, &ctx.paths, &ctx.state, &Logger::quiet());
    if let Ok(palette) = palette_result {
        println!(
            "\n  {}  {}\n",
            "".red().bold(),
            utils::capitalize(&palette.name).bold()
        );

        println!(
            "  {}",
            "Core Vs. Syntax:".bold().color_code_fg(&palette.comment)
        );
        palette.core_and_syntax_colors();

        println!(
            "\n  {}",
            "Terminal Colors:".bold().color_code_fg(&palette.comment)
        );
        palette.ansi_grid();

        println!(
            "\n  {}",
            "Sample code preview:"
                .bold()
                .color_code_fg(&palette.comment)
        );
        palette.preview_code();
    }

    Ok(())
}

/// Helper function to render generators list
fn render_generators_list(ctx: &IrisContext) {
    let enabled = &ctx.state.enabled_generators;
    if enabled.is_empty() {
        println!(
            "    {}",
            "No generators enabled. Use `iris gen auto` to find apps.".dimmed()
        );
    } else {
        for name in enabled {
            let icon = if ctx.registry.is_installed(name) {
                "󰄬".green()
            } else {
                "󰀦".yellow()
            };

            print!("    {} {}  ", icon, name.dimmed());
        }

        println!();
    }
}

/// Helper function to render sync block
fn render_sync_block(ctx: &IrisContext, is_sync: bool, iris_theme: &str, nvim_theme: &str) {
    println!();
    if is_sync {
        println!("  {}  {}", "󰄬".green(), "Sync with Neovim: OK".green());
    } else {
        ctx.log.warn("Out of sync with Neovim", 2);
        println!(
            "    {} {} {}  {} {}\n    {}",
            "Neovim:".dimmed(),
            nvim_theme.bright_yellow(),
            "󰄬".dimmed(),
            "Iris:".dimmed(),
            iris_theme.dimmed(),
            "󰚔  Run `iris sync` to update all configs".cyan().italic()
        );
    }
}

/// Helper function to render status in quiet mode
fn render_quiet(ctx: &IrisContext, current: &str, nvim_theme: &str, is_sync: bool) {
    let sync_icon = if is_sync {
        "󰄬".green()
    } else {
        "󰀦".yellow()
    };
    let gens = ctx
        .state
        .enabled_generators
        .iter()
        .map(|n| {
            if ctx.registry.is_installed(n) {
                n.normal()
            } else {
                n.strikethrough().dimmed()
            }
        })
        .map(|s| s.to_string())
        .collect::<Vec<_>>()
        .join(", ");

    println!(
        "\n{} Theme: {}\nPlugin manager: {}\nGenerators: {}",
        sync_icon,
        current.cyan().bold(),
        ctx.state.nvim,
        if gens.is_empty() {
            "none".dimmed()
        } else {
            gens.normal()
        }
    );

    if !is_sync {
        ctx.log
            .warn(&format!("Out of sync: Neovim is using `{}`", nvim_theme), 2);
    }
}
