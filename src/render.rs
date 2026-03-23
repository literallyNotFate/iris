use crate::models::palette::Palette;
use colored::*;

/// Display current theme colors
pub fn display_palette(p: &Palette, name: &str) {
    println!("\n{}", format!("   Theme: {}  ", name).red().bold());

    let section = |title: &str| println!("\n  {}", title.color_code_fg(&p.comment).bold());

    section("Core Palette");
    print_color("Background", &p.bg, p);
    print_color("Foreground", &p.fg, p);
    print_color("Selection ", &p.sel, p);
    print_color("Caret     ", &p.caret, p);

    section("Syntax Highlights");
    let syntax = [
        ("Keyword ", &p.keyword),
        ("Function", &p.func),
        ("String  ", &p.string),
        ("Constant", &p.constant),
        ("Comment ", &p.comment),
    ];

    for (label, color) in syntax {
        print_color(label, color, p);
    }

    section("Terminal Colors");
    print!("  ");
    for (i, color) in p.ansi.iter().enumerate() {
        print!("{}", "  ".on_color_code(color));
        if i == 7 {
            print!("\n  ");
        }
    }
    println!("\n");
}

fn print_color(label: &str, hex: &str, p: &Palette) {
    let block = "    ".on_color_code(hex);

    println!(
        "  {} {} {}",
        label.color_code_fg(&p.fg),
        block,
        hex.color_code_fg(&p.comment)
    );
}

/// Helper trait for colored to be able to work with hex
trait CustomColor {
    fn on_color_code(&self, hex: &str) -> ColoredString;
    fn color_code_fg(&self, hex: &str) -> ColoredString;
}

impl CustomColor for str {
    fn on_color_code(&self, hex: &str) -> ColoredString {
        let (r, g, b) = hex_to_rgb(hex);
        self.on_truecolor(r, g, b)
    }

    fn color_code_fg(&self, hex: &str) -> ColoredString {
        let (r, g, b) = hex_to_rgb(hex);
        self.truecolor(r, g, b)
    }
}

/// Helper function to convert hex to rgb
fn hex_to_rgb(hex: &str) -> (u8, u8, u8) {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return (128, 128, 128);
    }

    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
    (r, g, b)
}
