pub mod colors;
pub mod external;
pub mod strings;

pub use colors::{CustomColor, hex_to_rgb};
pub use external::*;
pub use strings::*;

/// Macro to skip the test case if app is not installed
#[macro_export]
macro_rules! skip_if_not_installed {
    ($executor:expr) => {
        if !$executor.is_installed() {
            println!(
                "cargo:warning=Skipping integration test for '{}': application not installed.",
                $executor.name()
            );

            return;
        }
    };
}
