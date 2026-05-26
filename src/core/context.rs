use super::IrisPaths;
use crate::{
    core::{NeovimBridge, Templater},
    log::Reporter,
    models::{PluginManager, State},
    modules::{Generator, GeneratorRegistry},
    utils,
};
use anyhow::{Context as _, Result};
use colored::*;
use std::{fs, path::PathBuf};

/// Application context with state and paths (config/cache/base)
#[derive(Clone)]
pub struct IrisContext {
    pub paths: IrisPaths,
    pub state: State,
    pub registry: GeneratorRegistry,
    pub templater: Templater,

    pub log: Reporter,
}

impl IrisContext {
    /// New context w/loading UIState from file
    pub fn new(log: Reporter) -> Result<Self> {
        let paths = IrisPaths::new()?;
        let user_templates: Option<PathBuf> = Some(paths.config.join("templates"));

        let state: State = if paths.state_file.exists() {
            let content: String = fs::read_to_string(&paths.state_file)
                .with_context(|| format!("Failed to read state at {:?}", &paths.state_file))?;
            serde_json::from_str(&content).context("Failed to parse state.json")?
        } else {
            State::default()
        };

        let ctx = Self {
            paths,
            state,
            registry: GeneratorRegistry::new(),
            log,
            templater: Templater::new(user_templates),
        };
        Ok(ctx)
    }

    /// Switch to specifc theme
    pub fn update(&mut self, name: &str) -> Result<()> {
        self.state.set_theme(name);
        self.paths.ensure_dirs()?;

        let json: String = self.state.to_json()?;
        fs::write(&self.paths.state_file, json)
            .with_context(|| format!("Failed to save state to {:?}", self.paths.state_file))?;

        fs::write(&self.paths.current_theme, name).with_context(|| {
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

    /// Function to resolve theme
    pub fn resolve_theme(
        &self,
        requested: Option<String>,
        fallback_enabled: bool,
    ) -> Result<(String, bool)> {
        let target: Option<String> = requested
            .map(|s| s.to_lowercase())
            .filter(|s| !s.is_empty())
            .or_else(|| {
                let current: String = self.state.current_theme.to_lowercase();
                if current.is_empty() {
                    None
                } else {
                    Some(current)
                }
            });

        match target {
            Some(name) if self.is_theme_available(&name) => Ok((name, false)),
            _ if fallback_enabled && !self.state.fallback_theme.is_empty() => {
                let fb: String = self.state.fallback_theme.clone();
                if self.is_theme_available(&fb) {
                    Ok((fb, true))
                } else {
                    anyhow::bail!(
                        "Requested theme unavailable and fallback `{}` not found.",
                        utils::capitalize(&fb).yellow()
                    )
                }
            }
            Some(name) => anyhow::bail!(
                "Theme `{}` is unavailable.",
                utils::capitalize(&name).yellow()
            ),
            None => anyhow::bail!("No theme specified and no global theme active."),
        }
    }

    /// Check whether requested theme is available
    fn is_theme_available(&self, name: &str) -> bool {
        if NeovimBridge::check_theme_exists(name, &self.state) {
            return true;
        }

        if self.state.manager == PluginManager::Default {
            let is_builtin: bool = NeovimBridge::get_builtin_themes().iter().any(|t| t == name);
            return is_builtin || self.paths.is_theme_cached(name);
        }

        false
    }

    /// Theme available wrapper with bail
    pub fn validate_theme_exists(&self, name: &str) -> anyhow::Result<()> {
        if self.is_theme_available(name) {
            Ok(())
        } else {
            anyhow::bail!(
                "Theme `{}` not found in cache or Neovim built-ins.",
                utils::capitalize(&name).yellow().bold()
            )
        }
    }

    /// Resolve generator or throw an error
    pub fn resolve_generator(&self, name: &str) -> Result<&dyn Generator> {
        self.registry.get(name).ok_or_else(|| {
            anyhow::anyhow!(
                "Unknown generator: `{}`. Run `{}` for available tools.",
                name.red().bold(),
                "iris gen list".cyan().italic()
            )
        })
    }

    /// Check if there are any generators with broken configs
    pub fn is_any_config_broken(&self) -> bool {
        self.registry.all().iter().any(|g| {
            g.health_check(&self.paths, &self.state.current_theme)
                .is_error()
        })
    }
}

/// Unit-tests for context
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::tests::create_test_context;

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

    #[test]
    fn should_resolve_theme_explicit_exists() {
        let (_temp, ctx) = create_test_context();

        fs::create_dir_all(&ctx.paths.themes).unwrap();
        fs::write(ctx.paths.themes.join("gruvbox.json"), "{}").unwrap();

        let (name, fallback) = ctx.resolve_theme(Some("Gruvbox".into()), false).unwrap();

        assert_eq!(name, "gruvbox");
        assert!(!fallback);
    }

    #[test]
    fn should_resolve_theme_fallback_to_current() {
        let (_temp, mut ctx) = create_test_context();
        ctx.state.current_theme = "nord".into();

        fs::create_dir_all(&ctx.paths.themes).unwrap();
        fs::write(ctx.paths.themes.join("nord.json"), "{}").unwrap();

        let (name, fallback) = ctx.resolve_theme(None, false).unwrap();

        assert_eq!(name, "nord");
        assert!(!fallback);
    }

    #[test]
    fn should_resolve_theme_use_fallback_theme_on_error() {
        let (_temp, mut ctx) = create_test_context();
        ctx.state.fallback_theme = "tokyonight".into();

        fs::create_dir_all(&ctx.paths.themes).unwrap();
        fs::write(ctx.paths.themes.join("tokyonight.json"), "{}").unwrap();

        let (name, fallback) = ctx.resolve_theme(Some("invalid".into()), true).unwrap();

        assert_eq!(name, "tokyonight");
        assert!(fallback);
    }

    #[test]
    fn should_check_if_theme_available_builtin() {
        let (_temp, mut ctx) = create_test_context();
        ctx.state.manager = PluginManager::Default;

        assert!(ctx.is_theme_available("habamax"));
        assert!(!ctx.is_theme_available("non-existent-theme-123"));
    }

    #[test]
    fn should_handle_resolve_generator_not_found() {
        let (_temp, ctx) = create_test_context();
        let result = ctx.resolve_generator("ghostty");
        assert!(result.is_err());

        let err_msg = format!("{:?}", result.err().unwrap());
        assert!(err_msg.contains("Unknown generator"));
    }
}
