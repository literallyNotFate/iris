use crate::service::ThemeService;
use anyhow::{Context, Result};
use colored::Colorize;
pub use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fmt, fs, path::PathBuf};

/// UI State of app which is being saved
#[derive(Default, Serialize, Deserialize, Debug, Clone)]
pub struct State {
    #[serde(default)]
    pub nvim: PluginState,

    #[serde(default)]
    pub theme: ThemeState,

    #[serde(default)]
    pub generators: GeneratorState,
}

/// All state related to theme
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ThemeState {
    #[serde(rename = "current")]
    pub current_theme: String,
    #[serde(rename = "previous")]
    pub previous_theme: Option<String>,
    #[serde(default = "retrobox", rename = "fallback")]
    pub fallback_theme: String,
}

/// All state related to generators
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct GeneratorState {
    #[serde(rename = "enabled")]
    pub enabled_generators: BTreeSet<String>,
}

/// All state related to plugins
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PluginState {
    pub manager: PluginManager,
}

/// Plugin manager for nvim (to find themes)
#[derive(Default, Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum PluginManager {
    #[default]
    Default,
    Lazy,
    Packer,
}

impl State {
    pub fn new(current_theme: String, enabled_generators: BTreeSet<String>) -> Self {
        Self {
            theme: ThemeState {
                current_theme,
                previous_theme: None,
                fallback_theme: retrobox(),
            },
            generators: GeneratorState { enabled_generators },
            ..Self::default()
        }
    }

    /// Casting to string (serialization) with anyhow error handling
    pub fn to_toml(&self) -> Result<String> {
        toml::to_string_pretty(self).context("Failed to serialize UI state to TOML!")
    }

    /// Set current theme to specific one, saving the previous one
    pub fn set_theme<S: Into<String>>(&mut self, name: S) {
        let new_theme: String = name.into().trim().to_lowercase();
        if self.theme.current_theme == new_theme {
            return;
        }

        self.theme.previous_theme = Some(self.theme.current_theme.clone());
        self.theme.current_theme = new_theme;
    }

    /// Set a new fallback theme after validating its existence via ThemeService.
    /// Returns true if the state actually changed
    pub fn set_fallback(&mut self, name: &str, service: &ThemeService) -> Result<bool> {
        let theme: String = crate::utils::capitalize(name.trim());
        if !service.exists(&theme, self) {
            anyhow::bail!(
                "Theme `{}` does not exist in `nvim` or cache.",
                theme.cyan().bold()
            );
        }

        let lower_theme: String = theme.to_lowercase();
        if self.theme.fallback_theme != lower_theme {
            self.theme.fallback_theme = lower_theme;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Toggles between current and previous theme.
    /// If no previous theme exists, falls back to `fallback_theme`.
    /// Returns the name of the newly activated theme.
    pub fn toggle_theme(&mut self) -> String {
        let target_theme = match &self.theme.previous_theme {
            Some(prev) => prev.clone(),
            None => self.theme.fallback_theme.clone(),
        };

        self.set_theme(target_theme);
        self.theme.current_theme.clone()
    }

    /// Enable generator
    pub fn enable_generator(&mut self, name: &str) -> bool {
        self.generators.enabled_generators.insert(name.to_string())
    }

    /// Disable generator
    pub fn disable_generator(&mut self, name: &str) -> bool {
        self.generators.enabled_generators.remove(name)
    }

    /// Full list replace (useful for MultiSelect)
    pub fn replace_enabled(&mut self, names: BTreeSet<String>) {
        self.generators.enabled_generators = names;
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

    /// Set a new plugin manager. Returns true if the state actually changed
    pub fn set_manager(&mut self, manager: PluginManager) -> bool {
        if self.nvim.manager != manager {
            self.nvim.manager = manager;
            true
        } else {
            false
        }
    }

    /// Check if generator is enabled
    pub fn is_enabled(&self, name: &str) -> bool {
        self.generators.enabled_generators.contains(name)
    }

    /// Wrapper to get rtp command
    pub fn get_rtp_command(&self) -> Option<String> {
        self.nvim.manager.get_rtp_command()
    }

    /// Save state to disk in TOML format
    pub fn save_to(&self, path: &PathBuf) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create directory structure: {}", parent.display())
            })?;
        }

        let toml_str: String = self.to_toml()?;
        fs::write(path, toml_str)
            .with_context(|| format!("Failed to write state file to: {}", path.display()))
    }

    /// Load state from file (Strict TOML parsing)
    pub fn load_from(path: &PathBuf) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read UI state at {:?}", path))?;

        toml::from_str(&content).context("Failed to parse state.toml")
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

impl Default for ThemeState {
    fn default() -> Self {
        Self {
            current_theme: String::new(),
            previous_theme: None,
            fallback_theme: retrobox(),
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

    /// Checks if plugin manager is set to default
    pub fn is_default(&self) -> bool {
        self == &PluginManager::Default
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
    use crate::utils::tests::mock_context;
    use tempdir::TempDir;

    #[test]
    fn should_test_application_state_logic() {
        let mut state = State::default();

        state.enable_generator("alacritty");
        state.enable_generator("wallust");
        assert!(state.is_enabled("alacritty"));

        state.enable_generator("alacritty");
        assert_eq!(state.generators.enabled_generators.len(), 2);

        state.toggle_generator("alacritty");
        assert!(!state.is_enabled("alacritty"));
    }

    #[test]
    fn should_handle_theme_transitions_and_toggle() {
        let mut state = State::default();
        state.theme.current_theme = "melange".to_string();
        state.theme.fallback_theme = "retrobox".to_string();

        state.set_theme("gruvbox");
        assert_eq!(state.theme.current_theme, "gruvbox");
        assert_eq!(state.theme.previous_theme, Some("melange".to_string()));

        let next = state.toggle_theme();
        assert_eq!(next, "melange");
    }

    #[test]
    fn should_fallback_on_toggle_if_no_previous_theme() {
        let mut state = State::default();
        state.theme.current_theme = "melange".to_string();
        state.theme.fallback_theme = "retrobox".to_string();
        state.theme.previous_theme = None;

        let result = state.toggle_theme();
        assert_eq!(result, "retrobox");
        assert_eq!(state.theme.current_theme, "retrobox");
        assert_eq!(state.theme.previous_theme, Some("melange".to_string()));
    }

    #[test]
    fn should_handle_state_save_and_load_toml() {
        let temp_dir: TempDir = TempDir::new("iris_state_toml_test").unwrap();
        let file_path: PathBuf = temp_dir.path().join("state.toml");
        let mut state: State = State {
            nvim: PluginState {
                manager: PluginManager::default(),
            },
            theme: ThemeState {
                current_theme: "melange".into(),
                previous_theme: Some("nord".into()),
                fallback_theme: "retrobox".into(),
            },
            generators: GeneratorState {
                enabled_generators: BTreeSet::new(),
            },
        };

        state.enable_generator("kitty");
        state.save_to(&file_path).expect("Failed to save TOML");

        let loaded = State::load_from(&file_path).expect("Failed to load TOML");
        assert_eq!(loaded.theme.current_theme, "melange");
        assert_eq!(loaded.theme.fallback_theme, "retrobox");
        assert_eq!(loaded.theme.previous_theme, Some("nord".to_string()));
        assert!(loaded.is_enabled("kitty"));
    }

    #[test]
    fn should_handle_missing_fields_gracefully() {
        let temp_dir: TempDir = TempDir::new("iris_compat_test").unwrap();
        let file_path: PathBuf = temp_dir.path().join("old_state.toml");
        let old_raw_toml = r#"
            [theme]
            current = "melange"

            [generator]
            enabled = ["alacritty"]
        "#;
        fs::write(&file_path, old_raw_toml).unwrap();
        let loaded: State =
            State::load_from(&file_path).expect("Should parse even without new fields");

        assert_eq!(loaded.theme.current_theme, "melange");
        assert_eq!(loaded.theme.previous_theme, None);
        assert_eq!(loaded.nvim.manager, PluginManager::Default);
        assert_eq!(loaded.theme.fallback_theme, "retrobox");
    }

    #[test]
    fn should_use_correct_defaults_on_manual_default_call() {
        let state: State = State::default();
        assert_eq!(state.theme.fallback_theme, "retrobox");
        assert_eq!(state.theme.current_theme, "");
        assert_eq!(state.theme.previous_theme, None);
    }

    #[test]
    fn should_handle_load_or_default_logic() {
        let temp_dir: TempDir = TempDir::new("iris_non_existent").unwrap();
        let file_path: PathBuf = temp_dir.path().join("not_found.toml");

        let state = State::load_or_default(&file_path);
        assert_eq!(state.theme.fallback_theme, "retrobox");
        assert!(state.generators.enabled_generators.is_empty());

        fs::write(&file_path, "invalid content").unwrap();
        let state_err = State::load_or_default(&file_path);
        assert_eq!(state_err.theme.fallback_theme, "retrobox");
    }

    #[test]
    fn should_persist_custom_fallback_theme() {
        let temp_dir: TempDir = TempDir::new("iris_fallback_test").unwrap();
        let file_path: PathBuf = temp_dir.path().join("state.toml");

        let mut state: State = State::default();
        state.theme.fallback_theme = "tokyonight".to_string();
        state.save_to(&file_path).unwrap();

        let loaded: State = State::load_from(&file_path).unwrap();
        assert_eq!(loaded.theme.fallback_theme, "tokyonight");
    }

    #[test]
    fn should_handle_to_toml_logic() {
        let mut state = State::default();
        state.set_theme("gruvbox");

        let toml_output = state.to_toml().unwrap();
        assert!(toml_output.contains("current = \"gruvbox\""));
    }

    #[test]
    fn should_persist_nvim_manager() {
        let temp_dir: TempDir = TempDir::new("iris_enum_test").unwrap();
        let file_path: PathBuf = temp_dir.path().join("state.toml");

        let mut state = State::default();
        state.nvim.manager = PluginManager::Lazy;
        state.save_to(&file_path).unwrap();

        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("manager = \"lazy\""));

        let loaded = State::load_from(&file_path).unwrap();
        assert_eq!(loaded.nvim.manager, PluginManager::Lazy);
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

    #[test]
    fn should_set_nvim_manager_and_return_changed_flag() {
        let mut state = State::default();
        assert_eq!(state.nvim.manager, PluginManager::Default);

        let changed = state.set_manager(PluginManager::Lazy);
        assert!(changed);
        assert_eq!(state.nvim.manager, PluginManager::Lazy);

        let changed_again = state.set_manager(PluginManager::Lazy);
        assert!(!changed_again);
    }

    #[test]
    fn should_validate_and_set_fallback_theme() {
        let (_, ctx) = mock_context();
        let mut state = State::default();
        let service = ThemeService::new(&ctx.paths, &ctx.log);

        state.theme.fallback_theme = "retrobox".to_string();
        let res = state.set_fallback("retrobox", &service);
        if let Ok(changed) = res {
            assert!(!changed);
        }
    }
}
