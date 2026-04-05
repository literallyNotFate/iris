pub mod colors;

pub use colors::{CustomColor, hex_to_rgb};

/// Helper function to capitalize the first letter of string
pub fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
        .unwrap_or_default()
}
