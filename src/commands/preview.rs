use crate::{
    core::IrisContext, log::Logger, models::Theme, service::ThemeService, utils::CustomColor,
};
use colored::*;

/// Handle theme preview command
pub fn exec(requested_theme: Option<String>, ctx: &IrisContext) -> anyhow::Result<()> {
    let (theme_name, is_fallback) = ctx.resolve_theme(requested_theme.clone(), true)?;

    let quiet: Logger = Logger::silent();
    let service: ThemeService = ThemeService::new(&ctx.paths, &quiet);
    let theme_obj: Theme = service.load_theme(&theme_name, false, true, &ctx.state)?;

    println!();
    render_header(&theme_obj.name, is_fallback, requested_theme);

    let label = |s: &str| s.bold().color_code_fg(&theme_obj.colors.comment);

    println!("  {}", label("Core & Syntax:"));
    theme_obj.colors.core_and_syntax_colors();

    println!("\n  {}", label("Terminal ANSI Grid:"));
    theme_obj.colors.ansi_grid();

    println!("\n  {}", label("Syntax Highlight Preview:"));
    theme_obj.colors.preview_code();

    println!("\n");
    Ok(())
}

/// Helper function to render header for preview
fn render_header(theme: &str, fallback: bool, requested: Option<String>) {
    if fallback {
        if let Some(req) = requested {
            println!(
                "{}  Theme `{}` not found, showing fallback",
                "󰁯".blue(),
                req.dimmed()
            );
        } else {
            println!("     {} Showing active system theme", "✓".blue());
        }
    }

    println!(
        "{}  {} {}",
        "".magenta().bold(),
        "Theme Preview:".bold(),
        theme.magenta().bold()
    );

    println!();
}
