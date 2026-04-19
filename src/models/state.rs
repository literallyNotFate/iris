use crate::models::NvimStrategy;
use anyhow::{Context, Result};
pub use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fs, path::PathBuf};

/// UI State of app which is being saved
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct State {
    pub current_theme: String,
    pub enabled_generators: BTreeSet<String>,

    #[serde(default = "retrobox")]
    pub fallback_theme: String,

    #[serde(default)]
    pub nvim: NvimStrategy,
}

impl State {
    pub fn new(current_theme: String, enabled_generators: BTreeSet<String>) -> Self {
        Self {
            current_theme,
            enabled_generators,
            ..Self::default()
        }
    }

    /// Casting to string (serialization) with anyhow error handling
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).context("Failed to serialize UI state to JSON")
    }

    /// Set current theme to specific one
    pub fn set_theme<S: Into<String>>(&mut self, name: S) {
        self.current_theme = name.into().trim().to_lowercase();
    }

    /// Enable generator
    pub fn enable_generator(&mut self, name: &str) -> bool {
        self.enabled_generators.insert(name.to_string())
    }

    /// Disable generator
    pub fn disable_generator(&mut self, name: &str) -> bool {
        self.enabled_generators.remove(name)
    }

    /// Full list replace (useful for MultiSelect)
    pub fn replace_enabled(&mut self, names: BTreeSet<String>) {
        self.enabled_generators = names;
    }

    /// Toggles generator (on/off) and returns new state
    pub fn toggle_generator(&mut self, name: &str) -> bool {
        if self.is_enabled(name) {
            self.disable_generator(name);
            false
        } else {
            self.enable_generator(name);
            true
        }
    }

    /// Check if generator is enabled
    pub fn is_enabled(&self, name: &str) -> bool {
        self.enabled_generators.contains(name)
    }

    /// Wrapper to get rtp command
    pub fn get_rtp_command(&self) -> Option<String> {
        self.nvim.get_rtp_command()
    }

    /// Wrapper to path resolve for strategy
    pub fn nvim_plugins_path(&self) -> Option<PathBuf> {
        NvimStrategy::resolve_path(&self.nvim)
    }

    /// Save state to disk
    pub fn save_to(&self, path: &PathBuf) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create directory structure: {}", parent.display())
            })?;
        }

        let json: String = self.to_json()?;
        fs::write(path, json)
            .with_context(|| format!("Failed to write state file to: {}", path.display()))
    }

    /// Load state from file
    pub fn load_from(path: &PathBuf) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read UI state at {:?}", path))?;

        serde_json::from_str(&content).context("Failed to parse state.json")
    }

    /// Load with reverting to default if problem occurs
    pub fn load_or_default(path: &PathBuf) -> Self {
        Self::load_from(path).unwrap_or_default()
    }
}

impl Default for State {
    fn default() -> Self {
        Self {
            current_theme: String::new(),
            enabled_generators: BTreeSet::new(),
            fallback_theme: retrobox(),
            nvim: NvimStrategy::default(),
        }
    }
}

/// Function to apply default fallback theme (for serde)
fn retrobox() -> String {
    "retrobox".to_string()
}

/// Unit-tests for application state
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempdir::TempDir;

    #[test]
    fn should_test_application_state_logic() {
        let mut state = State::default();

        state.enable_generator("alacritty");
        state.enable_generator("wallust");
        assert!(state.is_enabled("alacritty"));

        state.enable_generator("alacritty");
        assert_eq!(state.enabled_generators.len(), 2);

        state.toggle_generator("alacritty");
        assert!(!state.is_enabled("alacritty"));
    }

    #[test]
    fn should_handle_state_save_and_load() {
        let temp_dir: TempDir = TempDir::new("iris_state_test").unwrap();
        let file_path: PathBuf = temp_dir.path().join("state.json");
        let mut state: State = State {
            current_theme: "melange".into(),
            enabled_generators: BTreeSet::new(),
            fallback_theme: "retrobox".into(),
            nvim: NvimStrategy::Default,
        };

        state.enable_generator("kitty");
        state.save_to(&file_path).expect("Failed to save");

        let loaded = State::load_from(&file_path).expect("Failed to load");
        assert_eq!(loaded.current_theme, "melange");
        assert_eq!(loaded.fallback_theme, "retrobox");
        assert!(loaded.is_enabled("kitty"));
    }

    #[test]
    fn should_handle_missing_fields_gracefully() {
        let temp_dir: TempDir = TempDir::new("iris_compat_test").unwrap();
        let file_path: PathBuf = temp_dir.path().join("old_state.json");

        let old_raw_json = r#"{
                "current_theme": "melange",
                "enabled_generators": ["alacritty"]
            }"#;
        fs::write(&file_path, old_raw_json).unwrap();
        let loaded: State =
            State::load_from(&file_path).expect("Should parse even without new fields");

        assert_eq!(loaded.current_theme, "melange");
        assert_eq!(loaded.nvim, NvimStrategy::Default);
        assert_eq!(loaded.fallback_theme, "retrobox");
    }

    #[test]
    fn should_use_correct_defaults_on_manual_default_call() {
        let state: State = State::default();
        assert_eq!(state.fallback_theme, "retrobox");
        assert_eq!(state.current_theme, "");
    }

    #[test]
    fn should_handle_load_or_default_logic() {
        let temp_dir: TempDir = TempDir::new("iris_non_existent").unwrap();
        let file_path: PathBuf = temp_dir.path().join("not_found.json");

        let state = State::load_or_default(&file_path);
        assert_eq!(state.fallback_theme, "retrobox");
        assert!(state.enabled_generators.is_empty());

        fs::write(&file_path, "invalid json").unwrap();
        let state_err = State::load_or_default(&file_path);
        assert_eq!(state_err.fallback_theme, "retrobox");
    }

    #[test]
    fn should_persist_custom_fallback_theme() {
        let temp_dir: TempDir = TempDir::new("iris_fallback_test").unwrap();
        let file_path: PathBuf = temp_dir.path().join("state.json");

        let mut state: State = State::default();
        state.fallback_theme = "tokyonight".to_string();
        state.save_to(&file_path).unwrap();

        let loaded: State = State::load_from(&file_path).unwrap();
        assert_eq!(loaded.fallback_theme, "tokyonight");
    }

    #[test]
    fn should_handle_to_json_logic() {
        let mut state = State::default();
        state.set_theme("gruvbox");

        let json = state.to_json().unwrap();
        assert!(json.contains('\n'));
    }

    #[test]
    fn should_persist_nvim_strategy() {
        let temp_dir: TempDir = TempDir::new("iris_enum_test").unwrap();
        let file_path: PathBuf = temp_dir.path().join("state.json");

        let mut state = State::default();
        state.nvim = NvimStrategy::Lazy;
        state.save_to(&file_path).unwrap();

        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("lazy"));

        let loaded = State::load_from(&file_path).unwrap();
        assert_eq!(loaded.nvim, NvimStrategy::Lazy);
    }
}
