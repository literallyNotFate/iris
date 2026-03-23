use crate::models::palette::Palette;
use colored::*;

/// Display current theme colors
pub fn display_palette(p: &Palette, name: &str) {
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
