use anyhow::{Context, Result};
pub use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, path::PathBuf};

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
    pub fn set_theme(&mut self, name: &str) {
        self.current_theme = name.to_string();
    }

    /// Enable generator
    pub fn enable_generator(&mut self, name: String) {
        self.enabled_generators.insert(name);
    }

    /// Disable generator
    pub fn disable_generator(&mut self, name: &str) {
        self.enabled_generators.remove(name);
    }

    /// Check if generator is enabled
    pub fn is_enabled(&self, name: &str) -> bool {
        self.enabled_generators.contains(name)
    }

    /// Save state to disk
    pub fn save_to(&self, path: &PathBuf) -> Result<()> {
        let json: String = self.to_json()?;
        std::fs::write(path, json).with_context(|| format!("Failed to save state to {:?}", path))
    }

    /// Load state from file
    pub fn load_from(path: &PathBuf) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let content: String = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read UI state at {:?}", path))?;
        let state: State =
            serde_json::from_str(&content).with_context(|| "Failed to parse state.json")?;

        Ok(state)
    }
}
