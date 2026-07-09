pub mod bottom;
pub mod btop;

use super::Generator;
use std::sync::Arc;

pub fn get_all() -> Vec<Arc<dyn Generator>> {
    vec![
        Arc::new(btop::BtopGenerator),
        Arc::new(bottom::BottomGenerator),
    ]
}
