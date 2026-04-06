pub mod tmux;

use super::Generator;

pub fn get_all() -> Vec<Box<dyn Generator>> {
    vec![Box::new(tmux::TmuxGenerator)]
}
