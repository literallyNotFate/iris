use crate::{core::IrisContext, log::Reporter, models::Palette, utils::CustomColor};
use colored::*;

/// Handle theme preview command
pub fn exec(requested_theme: Option<String>, ctx: &IrisContext) -> anyhow::Result<()> {
    let (theme_name, is_fallback) = ctx.resolve_theme(requested_theme.clone(), true)?;

    let palette: Palette = Palette::fetch(
        &theme_name,
        false,
        false,
        &ctx.paths,
        &ctx.state,
        &Reporter::quiet(),
    )?;
    println!();
    render_header(&palette, is_fallback, requested_theme);

    let label = |s: &str| s.bold().color_code_fg(&palette.comment);

    println!("  {}", label("Core & Syntax:"));
    palette.core_and_syntax_colors();

    println!("\n  {}", label("Terminal ANSI Grid:"));
    palette.ansi_grid();

    println!("\n  {}", label("Syntax Highlight Preview:"));
    palette.preview_code();

    println!("\n");
    Ok(())
}

/// Helper function to render header for preview
fn render_header(p: &Palette, fallback: bool, requested: Option<String>) {
    if fallback {
        if let Some(req) = requested {
            println!(
                "{}  Theme `{}` not found, showing fallback",
                "󰁯".blue(),
                req.dimmed()
            );
        } else {
            println!("     {} Showing active system theme", "󰄬".blue());
        }
    }

    println!(
        "{}  {} {}",
        "".magenta().bold(),
        "Theme Preview:".bold(),
        p.name.magenta().bold()
    );

    println!();
}
