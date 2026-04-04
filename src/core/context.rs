use super::IrisPaths;
use crate::{models::State, modules::GeneratorRegistry};
use anyhow::{Context as _, Result};

/// Application context with state and paths (config/cache/base)
pub struct IrisContext {
    pub paths: IrisPaths,
    pub state: State,
    pub registry: GeneratorRegistry,
}

impl IrisContext {
    /// New context w/loading UIState from file
    pub fn new() -> Result<Self> {
        let paths = IrisPaths::new()?;

        let state = if paths.state_file.exists() {
            let content = std::fs::read_to_string(&paths.state_file)
                .with_context(|| format!("Failed to read state at {:?}", &paths.state_file))?;
            serde_json::from_str(&content).with_context(|| "Failed to parse state.json")?
        } else {
            State::default()
        };

        let ctx = Self {
            paths,
            state,
            registry: GeneratorRegistry::new(),
        };
        Ok(ctx)
    }

    /// Switch to specifc theme
    pub fn update(&mut self, name: &str) -> Result<()> {
        self.state.set_theme(name);
        self.paths.ensure_dirs()?;

        let json: String = self.state.to_json()?;
        std::fs::write(&self.paths.state_file, json)
            .with_context(|| format!("Failed to save state to {:?}", self.paths.state_file))?;

        std::fs::write(&self.paths.current_theme, name).with_context(|| {
            format!(
                "Failed to update theme cache at {:?}",
                self.paths.current_theme
            )
        })?;
        Ok(())
    }

    /// Saves current state of application to a file
    pub fn save(&self) -> Result<()> {
        self.paths.ensure_dirs()?;
        self.state.save_to(&self.paths.state_file)?;
        Ok(())
    }
}
