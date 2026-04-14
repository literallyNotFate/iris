pub mod bat;
pub mod fzf;
pub mod rules;
pub mod yazi;

use super::Generator;
use std::sync::Arc;

pub fn get_all() -> Vec<Arc<dyn Generator>> {
    vec![
        Arc::new(bat::BatGenerator),
        Arc::new(yazi::YaziGenerator),
        Arc::new(fzf::FzfGenerator),
    ]
}
