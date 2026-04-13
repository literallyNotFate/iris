pub mod bat;
pub mod fzf;
pub mod rules;
pub mod yazi;

use super::Generator;

pub fn get_all() -> Vec<Box<dyn Generator>> {
    vec![
        Box::new(bat::BatGenerator),
        Box::new(yazi::YaziGenerator),
        Box::new(fzf::FzfGenerator),
    ]
}
