use crate::models::UIState;
use std::fs;
use std::path::PathBuf;

/// Base path
pub fn get_base_path() -> PathBuf {
    std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| dirs::home_dir().unwrap().join(".config"))
        .join("iris")
}

/// Create folder if not found
pub fn setup_folders() {
    let base = get_base_path();
    let cache = dirs::home_dir().unwrap().join(".cache/iris");

    fs::create_dir_all(base.join("themes")).ok();
    fs::create_dir_all(cache).ok();
}

/// Save state to .config/
pub fn save_state(theme_name: &str) {
    let path = get_base_path().join("state.json");
    let state = UIState {
        current_theme: theme_name.to_string(),
    };
    fs::write(path, serde_json::to_string_pretty(&state).unwrap()).ok();
}
