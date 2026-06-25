pub mod colors;
pub mod external;
pub mod strings;

#[cfg(test)]
pub mod tests;

pub use colors::{CustomColor, hex_to_rgb};
pub use external::*;
pub use strings::*;
