use super::IrisPaths;
use crate::{models::State, modules::GeneratorRegistry, ui::Logger};
use anyhow::{Context as _, Result};

/// Application context with state and paths (config/cache/base)
pub struct IrisContext {
    pub paths: IrisPaths,
    pub state: State,
    pub registry: GeneratorRegistry,

    pub log: Logger,
}

impl IrisContext {
    /// New context w/loading UIState from file
    pub fn new(log: Logger) -> Result<Self> {
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
            log,
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

/// Unit-tests for context
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::create_test_context;
    use std::fs;

    #[test]
    fn should_handle_context_update_theme_persistence() {
        let (_tmp, mut ctx) = create_test_context();
        let theme_name = "melange";

        ctx.update(theme_name)
            .expect("Should update theme without errors");

        assert_eq!(ctx.state.current_theme, theme_name.to_string());

        let state_content = fs::read_to_string(&ctx.paths.state_file).unwrap();
        assert!(state_content.contains(theme_name));

        let current_theme_content = fs::read_to_string(&ctx.paths.current_theme).unwrap();
        assert_eq!(current_theme_content, theme_name);
    }

    #[test]
    fn should_handle_context_save_state() {
        let (_tmp, mut ctx) = create_test_context();
        ctx.state.enable_generator("yazi");
        ctx.save().expect("Should save state");

        let content = fs::read_to_string(&ctx.paths.state_file).unwrap();
        assert!(content.contains("yazi"));
    }

    #[test]
    fn should_handle_loading_context_from_existing_file() {
        let (_tmp, ctx_orig) = create_test_context();
        let mut ctx = ctx_orig;
        ctx.update("gruvbox").unwrap();

        let state_json = fs::read_to_string(&ctx.paths.state_file).unwrap();
        let loaded_state: State = serde_json::from_str(&state_json).unwrap();

        assert_eq!(loaded_state.current_theme, "gruvbox".to_string());
    }
}
