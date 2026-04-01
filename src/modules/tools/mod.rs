pub mod bat;
pub mod btop;
pub mod fzf;
pub mod yazi;

use super::ConfigGenerator;

pub fn get_all() -> Vec<Box<dyn ConfigGenerator>> {
    vec![
        Box::new(bat::BatGenerator),
        Box::new(yazi::YaziGenerator),
        Box::new(fzf::FzfGenerator),
        Box::new(btop::BtopGenerator),
    ]
}
