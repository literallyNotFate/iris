use crate::{infra::NeovimBridge, log::Logger, models::Palette, service::ThemeService};
use colored::Colorize;
use dialoguer::{Select, console::Term};

/// Handle select theme command
pub fn exec(ctx: &mut crate::core::IrisContext) -> anyhow::Result<()> {
    println!("\n{}  Scanning `nvim` themes...", "󱑠".yellow());

    let builtins: Vec<String> = NeovimBridge::builtin_themes()
        .iter()
        .map(|s| s.to_string())
        .collect();
    let mut cached: Vec<String> = ctx.paths.cached_themes().unwrap_or_default();
    let all_names: Vec<String> = NeovimBridge::installed_themes()?.to_vec();

    if all_names.is_empty() {
        println!(
            "{}  {}",
            "󰀦".red().bold(),
            "No themes found! Check your `nvim` configuration.".red()
        );
        return Ok(());
    }

    let mut current_idx: usize = all_names
        .iter()
        .position(|n| n == &ctx.state.theme.current_theme)
        .unwrap_or(0);

    let term = Term::stdout();
    let quiet: Logger = Logger::silent();
    let service = ThemeService::new(&ctx.paths, &quiet);

    term.hide_cursor()?;
    let _guard = crate::guards::CursorGuard::new(&term);

    loop {
        let labels: Vec<String> = all_names
            .iter()
            .map(|name| {
                render_theme_line(
                    name,
                    &ctx.state.theme.current_theme,
                    &ctx.state.theme.fallback_theme,
                    &cached,
                    &builtins,
                )
            })
            .collect();

        term.clear_screen()?;
        println!(
            "\n {}  {}\n",
            "󰏘".magenta().bold(),
            "Iris Theme Manager".bold()
        );

        let selection = Select::with_theme(&crate::utils::colors::select_theme())
            .with_prompt("Search or select theme (Press Esc to exit)\n")
            .items(&labels)
            .default(current_idx)
            .interact_on_opt(&term)?;

        let index = match selection {
            Some(idx) => idx,
            None => break,
        };

        current_idx = index;
        let selected_name = &all_names[index];

        if !cached.contains(selected_name) {
            term.write_line(&format!(
                "\n  {}  Fetching remote theme: {}...",
                "󰚔".cyan(),
                selected_name
            ))?;

            let theme_obj = service.load_theme(selected_name, false, true, &ctx.state)?;
            cached.push(selected_name.clone());

            term.clear_screen()?;
            render_preview_flow(&theme_obj.name, &theme_obj.colors)?;
        } else {
            let theme_obj = service.load_theme(selected_name, false, false, &ctx.state)?;
            term.clear_screen()?;
            render_preview_flow(&theme_obj.name, &theme_obj.colors)?;
        }

        println!();

        let action = Select::with_theme(&crate::utils::colors::select_theme())
            .items(&vec![
                "Apply this theme",
                "Apply this theme (Parallel)",
                "Back to list",
                "Exit",
            ])
            .default(0)
            .interact_on_opt(&term)?;

        match action {
            Some(0) => {
                term.clear_screen()?;
                let final_theme = service.load_theme(selected_name, false, true, &ctx.state)?;

                term.show_cursor()?;
                return crate::commands::apply_theme(&final_theme, false, ctx);
            }
            Some(1) => {
                term.clear_screen()?;
                let final_theme = service.load_theme(selected_name, false, true, &ctx.state)?;

                term.show_cursor()?;
                return crate::commands::apply_theme(&final_theme, true, ctx);
            }
            Some(2) => continue,
            _ => break,
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

/// Helper function to render theme line
fn render_theme_line(
    name: &str,
    current_theme: &str,
    fallback_theme: &str,
    cached_themes: &[String],
    builtin_themes: &[String],
) -> String {
    use colored::*;

    let is_active: bool = name == current_theme;
    let is_fallback: bool = name == fallback_theme;
    let is_cached: bool = cached_themes.contains(&name.to_string());
    let is_builtin: bool = builtin_themes.contains(&name.to_string());

    let padded_name: String = format!("{:<25}", name);
    let padded_cache = format!("{:<12}", if is_cached { "[cached]" } else { "[remote]" });
    let padded_type = format!("{:<12}", if is_builtin { "[builtin]" } else { "[lazy]" });

    let name_col = if is_active {
        padded_name.green().bold()
    } else {
        padded_name.normal()
    };
    let cache_col = if is_cached {
        padded_cache.dimmed()
    } else {
        padded_cache.yellow().dimmed()
    };
    let type_col = if is_builtin {
        padded_type.bright_red()
    } else {
        padded_type.bright_cyan()
    };

    let status_col = if is_active {
        "✓  active".green()
    } else if is_fallback {
        "󰁯  fallback".magenta()
    } else {
        "".normal()
    };

    format!("{} {} {} {}", name_col, cache_col, type_col, status_col)
}
