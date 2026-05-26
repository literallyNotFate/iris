pub(crate) mod item;

use crate::{
    commands::apply_theme,
    core::{IrisContext, NeovimBridge, ThemeOrchestrator},
    log::Reporter,
    models::{Palette, Theme},
    utils::colors::select_theme,
};
use colored::Colorize;
use dialoguer::{Select, console::Term};
use item::ThemeItem;

/// Handle select theme command
pub fn exec(ctx: &mut IrisContext) -> anyhow::Result<()> {
    println!("\n{}  Scanning Neovim themes...", "󱑠".yellow());

    let builtins: Vec<String> = NeovimBridge::get_builtin_themes();
    let cached: Vec<String> = ctx.paths.get_cached_themes().unwrap_or_default();
    let all_names: Vec<String> = NeovimBridge::get_all_themes()?;

    let items: Vec<ThemeItem> = all_names
        .into_iter()
        .map(|name| ThemeItem {
            is_cached: cached.contains(&name),
            is_builtin: builtins.contains(&name),
            is_active: name == ctx.state.current_theme,
            is_fallback: name == ctx.state.fallback_theme,
            name,
        })
        .collect();

    let labels: Vec<String> = items.iter().map(|i| i.render_label()).collect();
    let mut current_idx = items.iter().position(|i| i.is_active).unwrap_or(0);
    let term: Term = Term::stdout();

    let quiet_logger: Reporter = Reporter::quiet();
    let orchestrator = ThemeOrchestrator::new(&ctx.paths, &quiet_logger);

    loop {
        term.clear_screen()?;
        println!(
            "\n {}  {} \n",
            "󰏘".magenta().bold(),
            "Iris Theme Manager".bold()
        );

        let selection: Option<usize> = Select::with_theme(&select_theme())
            .with_prompt("Search or select theme\n")
            .items(&labels)
            .default(current_idx)
            .interact_on_opt(&term)?;

        if let Some(index) = selection {
            current_idx = index;
            let item = &items[index];

            if !item.is_cached {
                term.write_line(&format!(
                    "\n  {}  Fetching theme for {}...",
                    "󰚔".cyan(),
                    item.name
                ))?;
            }

            let theme_obj: Theme = orchestrator.load_theme(&item.name, false, false, &ctx.state)?;
            render_preview_flow(&theme_obj.name, &theme_obj.colors)?;
            println!();

            let action: Option<usize> = Select::with_theme(&select_theme())
                .items(&vec!["Apply this theme", "Back to list", "Exit"])
                .default(0)
                .interact_on_opt(&term)?;

            match action {
                Some(0) => {
                    term.clear_screen()?;
                    let final_theme: Theme =
                        orchestrator.load_theme(&item.name, false, true, &ctx.state)?;

                    return apply_theme(&final_theme, ctx);
                }
                Some(1) => continue,
                _ => break,
            }
        } else {
            break;
        }
    }

    Ok(())
}

/// Helper function to render theme preview
fn render_preview_flow(name: &str, palette: &Palette) -> anyhow::Result<()> {
    println!("\n  {}  Preview: {}", "".magenta(), name.bold());
    println!();
    palette.ansi_grid();

    println!("\n  {}  Code snippet: {}", "</>".green(), name.bold());
    palette.preview_code();

    println!();
    Ok(())
}
