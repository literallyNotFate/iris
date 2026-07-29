use crate::{core::IrisContext, service::ThemeService};
use colored::*;

/// Handle application status command
pub fn exec(ctx: &IrisContext) -> anyhow::Result<()> {
    let service: ThemeService = ThemeService::new(&ctx.paths, &ctx.log);
    let (nvim_theme, is_sync) = service.sync_status(&ctx.state);
    let current: String = ctx.state.theme.current_theme.clone();

    if !ctx.log.is_detailed() {
        render_quiet(ctx, &current, &nvim_theme, is_sync);
        return Ok(());
    }

    println!(
        "\n {}  {} {}",
        "󰗼".cyan().bold(),
        "Iris system status".bold(),
        format!("v{}", env!("CARGO_PKG_VERSION")).dimmed()
    );
    println!(
        "\n  {}  Active theme:    {}",
        "󰏘".red(),
        crate::utils::capitalize(&current).bold().blue()
    );
    println!(
        "  {}  Plugin Manager:  {}",
        "⚙".magenta(),
        ctx.state.nvim.manager
    );

    let cache_size: u64 = ctx.paths.get_size(&ctx.paths.themes);
    let cached_count: usize = std::fs::read_dir(&ctx.paths.themes)
        .map(|d| d.count())
        .unwrap_or(0);

    println!(
        "  {}  Config path:     {}",
        "󰉖".white(),
        crate::utils::pretty_path(&ctx.paths.config).bright_black()
    );
    println!(
        "  {}  Cache:           {} {} ({} files)",
        "󰉉".bright_black(),
        crate::utils::pretty_path(&ctx.paths.themes).bright_black(),
        format!("({})", crate::utils::format_size(cache_size))
            .yellow()
            .dimmed(),
        cached_count
    );

    println!("\n  {}  {}", "󰒓".yellow(), "Enabled generators:".bold());
    render_generators_list(ctx);

    render_sync_block(is_sync, &current, &nvim_theme);

    println!(
        "\n  {}  Run `{}` to see full colors and code preview.",
        "󰄶".blue(),
        "iris preview".cyan().bold()
    );

    println!();
    Ok(())
}

/// Helper function to render generators list
fn render_generators_list(ctx: &IrisContext) {
    let enabled = &ctx.state.generators.enabled_generators;
    if enabled.is_empty() {
        println!(
            "    {}",
            "No generators enabled. Use `iris gen auto` to find apps.".dimmed()
        );
    } else {
        for name in enabled {
            let icon = if ctx.registry.is_installed(name) {
                "✓".green()
            } else {
                "󰀦".yellow()
            };

            print!("    {} {}  ", icon, name.dimmed());
        }

        println!();
    }
}

/// Helper function to render sync block
fn render_sync_block(is_sync: bool, iris_theme: &str, nvim_theme: &str) {
    println!();
    if is_sync {
        println!("  {}  {}", "✓".green(), "Sync with Neovim: OK".green());
    } else {
        println!(
            "  {}  {}",
            "󰀦".yellow().bold(),
            "Out of sync with Neovim".yellow()
        );
        println!(
            "     {} {} {} {}",
            "Neovim:".dimmed(),
            nvim_theme.bright_yellow(),
            "󰁔".dimmed(),
            format!("Iris expects: {}", iris_theme).dimmed()
        );
        println!(
            "     {}",
            "󰚔  Run `iris sync` to update all configs".cyan().italic()
        );
    }
}

/// Helper function to render status in quiet mode
fn render_quiet(ctx: &IrisContext, current: &str, nvim_theme: &str, is_sync: bool) {
    let sync_icon = if is_sync {
        "✓".green()
    } else {
        "󰀦".yellow()
    };

    let gens = ctx
        .state
        .generators
        .enabled_generators
        .iter()
        .map(|n| {
            if ctx.registry.is_installed(n) {
                n.normal().to_string()
            } else {
                n.strikethrough().dimmed().to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(", ");

    println!(
        "\n{} Theme: {}\nPlugin manager: {}\nGenerators: {}",
        sync_icon,
        crate::utils::capitalize(current).cyan().bold(),
        ctx.state.nvim.manager,
        if gens.is_empty() {
            "none".dimmed().to_string()
        } else {
            gens
        }
    );

    if !is_sync {
        println!(
            "{} {}",
            "󰚔".cyan(),
            format!("Sync mismatch: Neovim is using `{}`", nvim_theme).dimmed()
        );
    }
    println!();
}
