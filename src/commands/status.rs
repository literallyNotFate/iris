use crate::{
    core::IrisContext,
    models::Palette,
    utils::{CustomColor, hex_to_rgb},
};
use anyhow::Result;
use colored::*;

/// Handle application status command
pub fn exec(ctx: &IrisContext) -> Result<()> {
    println!("\n{}\n", "Iris System Status".purple());
    let current = &ctx.state.current_theme;

    println!("  {} Active theme: {}", "●".blue(), current.bold().blue());
    println!(
        "  {} Enabled apps: {}",
        "●".yellow(),
        ctx.state.enabled_generators.join(", ").white()
    );
    println!(
        "  {} Config path:  {}",
        "●".white(),
        ctx.paths.config.display().to_string().bright_black()
    );

    if let Ok(nvim_theme) = Palette::current() {
        if nvim_theme.to_lowercase() != current.to_lowercase() {
            println!(
                "\n  {} {}",
                "⚠".yellow(),
                "Out of sync with Neovim".yellow().bold()
            );
            println!("    Neovim: {}", nvim_theme.bright_yellow());
            println!("    Iris:   {}", current.dimmed());
        } else {
            println!("\n  {} {}", "✔".green(), "Synchronized with Neovim".green());
        }
    }

    if let Ok(palette) = Palette::fetch(current) {
        display_palette(&palette, current);
    }

    Ok(())
}

/// Display current theme colors
fn display_palette(p: &Palette, name: &str) {
    println!("\n  {} {}\n", "   Theme:".bold(), name.red().bold());

    let syntax_labels = [
        ("Keyword ", &p.keyword),
        ("Function", &p.func),
        ("String  ", &p.string),
        ("Constant", &p.constant),
        ("Variable", &p.variable),
    ];

    let core_labels = [
        ("Background", &p.bg),
        ("Foreground", &p.fg),
        ("Selection ", &p.sel),
        ("Caret     ", &p.caret),
        ("Gutter    ", &p.gutter_fg),
    ];

    for i in 0..5 {
        print_row(core_labels[i], syntax_labels[i], p);
    }

    println!("\n  {}", "Terminal Colors".bold().color_code_fg(&p.comment));

    for row in 0..2 {
        print!("  ");
        for col in 0..8 {
            let idx = row * 8 + col;
            let color = &p.ansi[idx];
            let label = format!(" {:02} ", idx);
            print!("{}", label.on_color_code(color).black());
        }
        println!();
    }

    println!(
        "\n  {}",
        "Sample Text Preview:".bold().color_code_fg(&p.comment)
    );
    println!(
        "  {} {} {} {} {}",
        "fn".color_code_fg(&p.keyword),
        "main".color_code_fg(&p.func),
        "() {".color_code_fg(&p.fg),
        "\"Hello World\"".color_code_fg(&p.string),
        "};".color_code_fg(&p.fg)
    );
}

/// Helper function to print row
fn print_row(left: (&str, &String), right: (&str, &String), p: &Palette) {
    let format_col = |label: &str, hex: &str| {
        let (r, g, b) = hex_to_rgb(hex);
        let rgb_str = format!("({},{},{})", r, g, b);
        let block = "  ".on_color_code(hex);

        // {:<12} - for label (Background, Function etc.)
        // {:<9}  - for hex
        // {:<15} - for rgb tuple
        format!(
            "{:<12} {}  {:<9} {:<15}",
            label.color_code_fg(&p.fg),
            block,
            hex.color_code_fg(&p.comment),
            rgb_str.bright_black()
        )
    };

    println!(
        "  {} │ {}",
        format_col(left.0, left.1),
        format_col(right.0, right.1)
    );
}
