use super::IrisPaths;
use crate::{models::State, modules::ConfigGenerator};
use anyhow::{Context as _, Result};

/// Application context with state and paths (config/cache/base)
pub struct IrisContext {
    pub generators: Vec<Box<dyn ConfigGenerator>>,

    pub paths: IrisPaths,
    pub state: State,
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

        let mut ctx = Self {
            paths,
            state,
            generators: Vec::new(),
        };

        ctx.init_generators();
        Ok(ctx)
    }

    /// Helper function to add all supported generators
    fn init_generators(&mut self) {
        use crate::modules::{
            AlacrittyGenerator, BatGenerator, BtopGenerator, FzfGenerator, GhosttyGenerator,
            YaziGenerator,
        };

        let all: Vec<Box<dyn crate::modules::ConfigGenerator>> = vec![
            Box::new(GhosttyGenerator),
            Box::new(BatGenerator),
            Box::new(FzfGenerator),
            Box::new(BtopGenerator),
            Box::new(YaziGenerator),
            Box::new(AlacrittyGenerator),
        ];

        self.generators = all
            .into_iter()
            .filter(|g| {
                self.state
                    .enabled_generators
                    .contains(&g.name().to_string())
            })
            .collect();
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
        let json: String = self.state.to_json()?;

        std::fs::write(&self.paths.state_file, json)
            .with_context(|| format!("Failed to save state to {:?}", self.paths.state_file))?;
        Ok(())
    }
}
