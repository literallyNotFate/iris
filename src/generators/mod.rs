pub mod fzf;
pub mod ghostty;

use crate::models::Theme;

/// Apply themes to available programs
pub fn apply_enabled(theme: &Theme, enabled: &[String]) {
    for app in enabled {
        match app.as_str() {
            "ghostty" => ghostty::apply(theme),
            "fzf" => fzf::apply(theme),
            _ => println!("Unknown generator: {}", app),
        }
    }
}
