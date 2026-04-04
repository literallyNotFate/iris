use anyhow::{Context, Result};
pub use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fs, path::PathBuf};

/// UI State of app which is being saved
#[derive(Default, Serialize, Deserialize, Debug)]
pub struct State {
    pub current_theme: String,
    pub enabled_generators: BTreeSet<String>,
}

impl State {
    pub fn new(current_theme: String, enabled_generators: BTreeSet<String>) -> Self {
        Self {
            current_theme,
            enabled_generators,
        }
    }

    /// Casting to string (serialization) with anyhow error handling
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).context("Failed to serialize UI state to JSON")
    }

    /// Set current theme to specific one
    pub fn set_theme<S: Into<String>>(&mut self, name: S) {
        self.current_theme = name.into();
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

    /// Save state to disk
    pub fn save_to(&self, path: &PathBuf) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let json: String = self.to_json()?;
        fs::write(path, json).with_context(|| format!("Failed to save state to {:?}", path))
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
