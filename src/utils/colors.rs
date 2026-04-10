use colored::{ColoredString, Colorize};

/// Helper trait for colored to be able to work with hex
pub trait CustomColor {
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
pub fn hex_to_rgb(hex: &str) -> (u8, u8, u8) {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return (128, 128, 128);
    }

    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
    (r, g, b)
}

/// Unit-tests for color utility functions
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_parse_hex_to_rgb() {
        assert_eq!(hex_to_rgb("#ffffff"), (255, 255, 255));
        assert_eq!(hex_to_rgb("000000"), (0, 0, 0));
        assert_eq!(hex_to_rgb("#ff5500"), (255, 85, 0));
    }

    #[test]
    fn should_handle_wrong_hex() {
        assert_eq!(hex_to_rgb("short"), (128, 128, 128));
        assert_eq!(hex_to_rgb("#zzzzzz"), (0, 0, 0));
    }
}
