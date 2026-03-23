use crate::models::State;
use anyhow::{Context as _, Result};
use std::fs;
use std::path::PathBuf;

/// Application context with state and paths (config/cache/base)
pub struct AppContext {
    pub base_path: PathBuf,
    pub cache_path: PathBuf,
    pub state: State,
}

impl AppContext {
    /// New context w/loading UIState from file
    pub fn new() -> Result<Self> {
        let base_path: PathBuf = Self::default_base_path();
        let cache_path: PathBuf = Self::default_cache_path();
        let state_path: PathBuf = base_path.join("state.json");

        let state = if state_path.exists() {
            let content = fs::read_to_string(&state_path)
                .with_context(|| format!("Failed to read state at {:?}", state_path))?;
            serde_json::from_str(&content).with_context(|| "Failed to parse state.json")?
        } else {
            State::default()
        };

        Ok(Self {
            base_path,
            cache_path,
            state,
        })
    }

    /// Switch to specifc theme
    pub fn update(&mut self, name: &str) -> Result<()> {
        self.state.set_theme(name);

        let json = self.state.to_json()?;
        let state_path = self.base_path.join("state.json");

        fs::create_dir_all(&self.base_path)?;
        fs::write(&state_path, json)
            .with_context(|| format!("Failed to save state to {:?}", state_path))?;

        let theme_cache_path = self.cache_path.join("current_theme");

        fs::create_dir_all(&self.cache_path)?;
        fs::write(&theme_cache_path, name)
            .with_context(|| format!("Failed to update theme cache at {:?}", theme_cache_path))?;

        Ok(())
    }

    /// Get base path (.config)
    pub fn default_base_path() -> PathBuf {
        std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| dirs::home_dir().unwrap().join(".config"))
            .join("iris")
    }

    /// Get cache path
    pub fn default_cache_path() -> PathBuf {
        dirs::home_dir().unwrap().join(".cache/iris")
    }
}
