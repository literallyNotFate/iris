use anyhow::{Context, Result};
use colored::Colorize;
pub use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fmt, fs, path::PathBuf};

/// UI State of app which is being saved
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct State {
    pub current_theme: String,
    pub previous_theme: Option<String>,
    pub enabled_generators: BTreeSet<String>,

    #[serde(default = "retrobox")]
    pub fallback_theme: String,

    #[serde(default)]
    pub manager: PluginManager,
}

/// Plugin manager for nvim (to find themes)
#[derive(Default, Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
#[serde(tag = "type", content = "value")]
pub enum PluginManager {
    #[default]
    Default,
    Lazy,
    Packer,
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

    /// Set current theme to specific one, saving the previous one
    pub fn set_theme<S: Into<String>>(&mut self, name: S) {
        let new_theme: String = name.into().trim().to_lowercase();
        if self.current_theme == new_theme {
            return;
        }

        self.previous_theme = Some(self.current_theme.clone());
        self.current_theme = new_theme;
    }

    /// Toggles between current and previous theme.
    /// If no previous theme exists, falls back to `fallback_theme`.
    /// Returns the name of the newly activated theme.
    pub fn toggle_theme(&mut self) -> String {
        let target_theme = match &self.previous_theme {
            Some(prev) => prev.clone(),
            None => self.fallback_theme.clone(),
        };

        self.set_theme(target_theme);
        self.current_theme.clone()
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
        self.manager.get_rtp_command()
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

/// Function to apply default fallback theme (for serde)
fn retrobox() -> String {
    "retrobox".to_string()
}

impl Default for State {
    fn default() -> Self {
        Self {
            current_theme: String::new(),
            previous_theme: None,
            enabled_generators: BTreeSet::new(),
            fallback_theme: retrobox(),
            manager: PluginManager::default(),
        }
    }
}

impl PluginManager {
    pub fn all() -> [PluginManager; 3] {
        [
            PluginManager::Lazy,
            PluginManager::Packer,
            PluginManager::Default,
        ]
    }

    /// Returns relative subpath for plugins (clean string)
    pub fn plugin_subdirectory(&self) -> Option<&'static str> {
        match self {
            PluginManager::Lazy => Some("lazy"),
            PluginManager::Packer => Some("site/pack/packer/start"),
            PluginManager::Default => None,
        }
    }

    /// Generates Lua-command for runtimepath extension
    pub fn get_rtp_command(&self) -> Option<String> {
        let folder: &str = self.plugin_subdirectory()?;
        Some(format!(
            "lua local p = vim.fn.stdpath('data') .. '/{}' for _, dir in ipairs(vim.fn.expand(p .. '/*', false, true)) do vim.opt.rtp:append(dir) end",
            folder
        ))
    }
}

impl fmt::Display for PluginManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            PluginManager::Lazy => "Lazy.nvim".cyan().bold(),
            PluginManager::Packer => "Packer.nvim".yellow().bold(),
            PluginManager::Default => "Built-in".red().bold(),
        };
        write!(f, "{}", text)
    }
}

/// Unit-tests for application state and plugin manager
#[cfg(test)]
mod tests {
    use super::*;
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
    fn should_handle_theme_transitions_and_toggle() {
        let mut state = State::default();
        state.current_theme = "melange".to_string();
        state.fallback_theme = "retrobox".to_string();

        state.set_theme("gruvbox");
        assert_eq!(state.current_theme, "gruvbox");
        assert_eq!(state.previous_theme, Some("melange".to_string()));

        state.set_theme("gruvbox");
        assert_eq!(state.previous_theme, Some("melange".to_string()));

        let next = state.toggle_theme();
        assert_eq!(next, "melange");
        assert_eq!(state.current_theme, "melange");
        assert_eq!(state.previous_theme, Some("gruvbox".to_string()));

        let next_again = state.toggle_theme();
        assert_eq!(next_again, "gruvbox");
        assert_eq!(state.current_theme, "gruvbox");
        assert_eq!(state.previous_theme, Some("melange".to_string()));
    }

    #[test]
    fn should_fallback_on_toggle_if_no_previous_theme() {
        let mut state = State::default();
        state.current_theme = "melange".to_string();
        state.fallback_theme = "retrobox".to_string();
        state.previous_theme = None;

        let result = state.toggle_theme();
        assert_eq!(result, "retrobox");
        assert_eq!(state.current_theme, "retrobox");
        assert_eq!(state.previous_theme, Some("melange".to_string()));
    }

    #[test]
    fn should_handle_state_save_and_load() {
        let temp_dir: TempDir = TempDir::new("iris_state_test").unwrap();
        let file_path: PathBuf = temp_dir.path().join("state.json");
        let mut state: State = State {
            current_theme: "melange".into(),
            enabled_generators: BTreeSet::new(),
            fallback_theme: "retrobox".into(),
            manager: PluginManager::Default,
            previous_theme: Some("nord".into()),
        };

        state.enable_generator("kitty");
        state.save_to(&file_path).expect("Failed to save");

        let loaded = State::load_from(&file_path).expect("Failed to load");
        assert_eq!(loaded.current_theme, "melange");
        assert_eq!(loaded.fallback_theme, "retrobox");
        assert_eq!(loaded.previous_theme, Some("nord".to_string()));
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
        assert_eq!(loaded.previous_theme, None);
        assert_eq!(loaded.manager, PluginManager::Default);
        assert_eq!(loaded.fallback_theme, "retrobox");
    }

    #[test]
    fn should_use_correct_defaults_on_manual_default_call() {
        let state: State = State::default();
        assert_eq!(state.fallback_theme, "retrobox");
        assert_eq!(state.current_theme, "");
        assert_eq!(state.previous_theme, None);
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
    fn should_persist_nvim_manager() {
        let temp_dir: TempDir = TempDir::new("iris_enum_test").unwrap();
        let file_path: PathBuf = temp_dir.path().join("state.json");

        let mut state = State::default();
        state.manager = PluginManager::Lazy;
        state.save_to(&file_path).unwrap();

        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("lazy"));

        let loaded = State::load_from(&file_path).unwrap();
        assert_eq!(loaded.manager, PluginManager::Lazy);
    }

    #[test]
    fn should_get_rtp_command_properly() {
        let manager = PluginManager::Lazy;
        let cmd = manager.get_rtp_command();

        assert!(cmd.is_some());
        assert!(cmd.unwrap().contains("rtp:append"));

        let default_manager = PluginManager::Default;
        assert!(default_manager.get_rtp_command().is_none());
    }
}
