pub mod alacritty;
pub mod ghostty;

use super::ConfigGenerator;

pub fn get_all() -> Vec<Box<dyn ConfigGenerator>> {
    vec![
        Box::new(alacritty::AlacrittyGenerator),
        Box::new(ghostty::GhosttyGenerator),
    ]
}
