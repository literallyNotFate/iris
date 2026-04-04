pub mod alacritty;
pub mod ghostty;

use super::Generator;

pub fn get_all() -> Vec<Box<dyn Generator>> {
    vec![
        Box::new(alacritty::AlacrittyGenerator),
        Box::new(ghostty::GhosttyGenerator),
    ]
}
