pub(crate) mod item;

use crate::{
    commands::apply_theme,
    core::IrisContext,
    log::Reporter,
    models::{NvimStrategy, Palette},
    utils::colors::select_theme,
};
use colored::Colorize;
use dialoguer::{Select, console::Term};
use item::ThemeItem;

/// Handle select theme command
pub fn exec(ctx: &mut IrisContext) -> anyhow::Result<()> {
    println!("\n{}  Scanning Neovim themes...", "󱑠".yellow());

    let builtins: Vec<String> = NvimStrategy::get_builtin_themes();
    let cached: Vec<String> = ctx.paths.get_cached_themes().unwrap_or_default();
    let all_names: Vec<String> = NvimStrategy::get_all_themes()?;

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
                    "\n  {}  Fetching palette for {}...",
                    "󰚔".cyan(),
                    item.name
                ))?;
            }

            let palette: Palette = Palette::fetch(
                &item.name,
                false,
                false,
                &ctx.paths,
                &ctx.state,
                &Reporter::quiet(),
            )?;

            render_preview_flow(&palette)?;
            println!();

            let action: Option<usize> = Select::with_theme(&select_theme())
                .items(&vec!["Apply this theme", "Back to list", "Exit"])
                .default(0)
                .interact_on_opt(&term)?;

            match action {
                Some(0) => {
                    term.clear_screen()?;

                    if !item.is_cached {
                        let cache_path = ctx
                            .paths
                            .palettes
                            .join(format!("{}.json", item.name.to_lowercase()));
                        Palette::save_to_cache(&cache_path, &palette)?;
                    }

                    return apply_theme(&palette, ctx);
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
fn render_preview_flow(palette: &Palette) -> anyhow::Result<()> {
    let name = &palette.name;

    println!("\n  {}  Preview: {}", "".magenta(), name.bold());
    println!();
    palette.ansi_grid();

    println!("\n  {}  Code snippet: {}", "</>".green(), name.bold());
    palette.preview_code();

    println!();
    Ok(())
}
