use anyhow::{Context, Result, anyhow};
use std::path::PathBuf;

/// Paths manager for application
pub struct IrisPaths {
    /// Points to: ~/.config/iris
    pub config: PathBuf,
    /// Points to: ~/.cache/iris
    pub cache: PathBuf,
    /// Points to: ~/.config/iris/state.json
    pub state_file: PathBuf,
    /// Points to: ~/.cache/iris/current_theme
    pub current_theme: PathBuf,
}

impl IrisPaths {
    pub fn new() -> Result<Self> {
        let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not find home directory"))?;

        let config = std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| home.join(".config"))
            .join("iris");

        let cache = std::env::var("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| home.join(".cache"))
            .join("iris");

        Ok(Self {
            state_file: config.join("state.json"),
            current_theme: cache.join("current_theme"),
            config,
            cache,
        })
    }

    /// Creates all folders for iris if there none
    pub fn ensure_dirs(&self) -> Result<()> {
        std::fs::create_dir_all(&self.config)
            .with_context(|| "Failed to create config directory")?;
        std::fs::create_dir_all(&self.cache).with_context(|| "Failed to create cache directory")?;
        Ok(())
    }
}
