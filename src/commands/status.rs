use crate::{
    core::IrisContext,
    models::Palette,
    utils::{CustomColor, hex_to_rgb},
};
use colored::*;

/// Handle application status command
pub fn exec(ctx: &IrisContext) -> anyhow::Result<()> {
    println!(
        "\n {}  {}",
        "󰄬".purple().bold(),
        "Iris System Status".bold()
    );
    println!("{}", " ─────────────────────────────────────────".dimmed());

    let current = &ctx.state.current_theme;

    println!("  {}  Active theme:  {}", "󰏘".blue(), current.bold().blue());
    println!(
        "  {}  Config path:   {}",
        "󰉖".white(),
        ctx.paths.config.display().to_string().bright_black()
    );

    println!("\n  {}  {}", "󰒓".yellow(), "Enabled Generators:".bold());

    let enabled = &ctx.state.enabled_generators;
    if enabled.is_empty() {
        println!(
            "    {}",
            "No generators enabled. Use 'iris gen auto' to find apps.".dimmed()
        );
    } else {
        for name in enabled {
            let status_icon = if ctx.registry.is_installed(name) {
                "󰄬".green()
            } else {
                "󰀦".yellow()
            };

            print!("    {} {}  ", status_icon, name.dimmed());
        }
        println!();
    }

    if let Ok(nvim_theme) = Palette::current() {
        println!();
        if nvim_theme.to_lowercase() != current.to_lowercase() {
            println!(
                "  {} {}",
                "󰀦".yellow().bold(),
                "Out of sync with Neovim".yellow().bold()
            );
            println!(
                "    {} {} {}",
                "Neovim:".dimmed(),
                nvim_theme.bright_yellow(),
                "󰄬".dimmed()
            );
            println!(
                "    {} {} {}",
                "Iris:  ".dimmed(),
                current.dimmed(),
                "󰚔 run 'iris sync'".cyan().italic()
            );
        } else {
            println!("  {} {}", "󰄬".green(), "Synchronized with Neovim".green());
        }
    }

    if let Ok(palette) = Palette::fetch(current) {
        println!();
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
