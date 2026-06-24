pub mod alacritty;
pub mod ghostty;
pub mod wezterm;

use super::Generator;
use std::sync::Arc;

pub fn get_all() -> Vec<Arc<dyn Generator>> {
    vec![
        Arc::new(alacritty::AlacrittyGenerator),
        Arc::new(ghostty::GhosttyGenerator),
        Arc::new(wezterm::WezTermGenerator),
    ]
}
