pub mod tmux;

use super::Generator;
use std::sync::Arc;

pub fn get_all() -> Vec<Arc<dyn Generator>> {
    vec![Arc::new(tmux::TmuxGenerator)]
}
