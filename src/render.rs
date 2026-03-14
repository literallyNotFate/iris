use crate::models::palette::Palette;
use colored::*;

/// Display current theme colors
pub fn display_palette(p: &Palette, name: &str) {
    println!(
        "\n{}",
        format!(" Theme: {} ", name)
            .on_bright_purple()
            .white()
            .bold()
    );

    println!("\n  {}", "Core Palette".bright_black().italic());

    print_color("Background", &p.bg);
    print_color("Foreground", &p.fg);
    print_color("Selection ", &p.sel);
    print_color("Caret     ", &p.caret);

    println!("\n  {}", "Syntax Highlights".bright_black().italic());
    let syntax = [
        ("Keyword ", &p.keyword),
        ("Function", &p.func),
        ("String  ", &p.string),
        ("Constant", &p.constant),
        ("Comment ", &p.comment),
    ];

    for (label, color) in syntax {
        print_color(label, color);
    }

    println!("\n  {}", "Terminal Colors".bright_black().italic());
    print!("  ");
    for (i, color) in p.ansi.iter().enumerate() {
        let block = "  ".on_color_code(color);
        print!("{}", block);
        if i == 7 {
            print!("\n  ");
        }
    }
    println!("\n");
}

fn print_color(label: &str, hex: &str) {
    let block = "    ".on_color_code(hex);
    println!("  {} {} {}", label.white(), block, hex.bright_black());
}

/// Helper trait for colored to be able to work with hex
trait CustomColor {
    fn on_color_code(&self, hex: &str) -> ColoredString;
}

impl CustomColor for str {
    fn on_color_code(&self, hex: &str) -> ColoredString {
        let r = u8::from_str_radix(&hex[1..3], 16).unwrap_or(0);
        let g = u8::from_str_radix(&hex[3..5], 16).unwrap_or(0);
        let b = u8::from_str_radix(&hex[5..7], 16).unwrap_or(0);
        self.on_truecolor(r, g, b)
    }
}
