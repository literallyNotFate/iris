pub mod colors;
pub mod status;

pub use colors::{CustomColor, hex_to_rgb};
pub use status::{Status, Task};

/// Helper function to capitalize the first letter of string
pub fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
        .unwrap_or_default()
}
