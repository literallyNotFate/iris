use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// UI State of app which is being saved
#[derive(Serialize, Deserialize, Debug)]
pub struct UIState {
    pub current_theme: String,
}

impl Default for UIState {
    fn default() -> Self {
        Self {
            current_theme: "melange".to_string(),
        }
    }
}

/// Theme
#[derive(Deserialize, Debug)]
pub struct Theme {
    pub name: String,
    pub colors: BTreeMap<String, String>,
    pub palette: BTreeMap<String, String>,
}
