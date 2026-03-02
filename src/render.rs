use crate::models::Theme;
use colored::*;

/// Display color palette of theme
pub fn display_palette(theme: &Theme) {
    println!(
        "\n{}",
        format!("── {} ──", theme.name).bold().bright_white()
    );

    let print_row = |range: std::ops::Range<u8>| {
        for i in range {
            let hex = theme
                .palette
                .get(&i.to_string())
                .map(|s| s.as_str())
                .unwrap_or("#000000");
            let rgb = parse_hex(hex);

            print!("{} {:2} ", "██".custom_color(rgb), i);
        }
        println!();
    };

    print_row(0..8); // Normal colors
    print_row(8..16); // Bright colors

    println!("{}", "──────────────────".bright_black());
    println!(
        "{} background: #{}",
        "  ".on_custom_color(parse_hex(
            theme
                .colors
                .get("background")
                .unwrap_or(&"#000000".to_string())
        )),
        theme.colors.get("background").unwrap_or(&"".to_string())
    );
    println!();
}

/// Parse hex to custom color
fn parse_hex(hex: &str) -> CustomColor {
    let hex = hex.trim_start_matches('#');
    CustomColor {
        r: u8::from_str_radix(&hex[0..2], 16).unwrap_or(0),
        g: u8::from_str_radix(&hex[2..4], 16).unwrap_or(0),
        b: u8::from_str_radix(&hex[4..6], 16).unwrap_or(0),
    }
}
