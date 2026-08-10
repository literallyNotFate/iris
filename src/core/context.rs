use crate::{
    core::IrisEngine,
    infra::*,
    log::Logger,
    models::{PluginManager, State, Theme},
    modules::{Generator, GeneratorRegistry},
    utils,
};
use anyhow::{Context as _, Result};
use colored::*;
use std::{collections::BTreeSet, fs, path::PathBuf};

#[cfg(test)]
use tempdir::TempDir;

/// Application context with state and paths (config/cache/base)
#[derive(Clone)]
pub struct IrisContext {
    pub paths: IrisPaths,
    pub state: State,
    pub registry: GeneratorRegistry,
    pub templater: Templater,

    pub log: Logger,
}

impl IrisContext {
    /// New context w/loading UIState from file
    pub fn new(log: Logger) -> Result<Self> {
        let paths = IrisPaths::new()?;
        let user_templates: Option<PathBuf> = Some(paths.config.join("templates"));
        let state: State = State::load_or_default(&paths.state_file);
        let templater: Templater = Templater::new(user_templates);

        Ok(Self {
            paths,
            state,
            registry: GeneratorRegistry::new(),
            templater,
            log,
        })
    }

    /// Creates engine fast using refs that are already in the context itself
    pub fn engine<'t>(&self, theme: &'t Theme) -> IrisEngine<'_, 't> {
        IrisEngine::new(&self.paths, &self.templater, theme)
    }

    /// Switch to specifc theme
    pub fn update(&mut self, name: &str) -> Result<()> {
        self.state.set_theme(name);
        self.paths.ensure_dirs()?;

        let toml: String = self.state.to_toml()?;
        fs::write(&self.paths.state_file, toml)
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
                let current: String = self.state.theme.current_theme.to_lowercase();
                if current.is_empty() {
                    None
                } else {
                    Some(current)
                }
            });

        match target {
            Some(name) if self.is_theme_available(&name) => Ok((name, false)),
            _ if fallback_enabled && !self.state.theme.fallback_theme.is_empty() => {
                let fb: String = self.state.theme.fallback_theme.clone();
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

        if self.state.nvim.manager == PluginManager::Default {
            let is_builtin = NeovimBridge::builtin_themes().iter().any(|&t| t == name);
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
            g.health_check(&self.paths, &self.state.theme.current_theme)
                .is_error()
        })
    }

    /// Get all available themes (builtin ones and cache) for autocompletion
    pub fn get_available_themes(&self) -> Result<Vec<String>> {
        let mut themes: BTreeSet<String> = BTreeSet::new();
        for theme in crate::infra::BUILTIN_NVIM_THEMES {
            themes.insert(theme.to_string());
        }

        if self.paths.themes.exists() {
            if let Ok(entries) = fs::read_dir(&self.paths.themes) {
                for theme_name in entries.flatten().filter_map(|entry| {
                    let path = entry.path();
                    if path.is_file() {
                        path.file_stem()?.to_str().map(|s| s.to_string())
                    } else {
                        None
                    }
                }) {
                    themes.insert(theme_name);
                }
            }
        }

        Ok(themes.into_iter().collect())
    }
}

/// Mocks
#[cfg(test)]
impl IrisContext {
    /// Default mock context
    pub fn mock() -> (TempDir, Self) {
        let def_templates = [
            ("terminals/ghostty", "mock-ghostty-content"),
            ("terminals/alacritty", "mock-alacritty-content"),
            ("terminals/kitty", "mock-kitty-content"),
            ("terminals/wezterm", "mock-wezterm-content"),
            ("prompts/starship", "mock-starship-content"),
            ("system/bottom", "mock-bottom-content"),
            ("system/btop", "mock-btop-content"),
            ("multiplexer/herdr", "mock-herdr-content"),
            ("multiplexer/tmux", "mock-tmux-content"),
            ("tools/yazi", "mock-yazi-content"),
            ("tools/fzf", "mock-fzf-content"),
            ("tools/bat", "mock-bat-content"),
        ];

        Self::with_templates(def_templates.to_vec())
    }

    /// Mock with premade templates
    pub fn with_templates(templates: Vec<(&str, &str)>) -> (TempDir, Self) {
        use crate::{
            infra::{IrisPaths, Templater},
            log::Logger,
            models::State,
            modules::GeneratorRegistry,
        };

        let temp_dir: TempDir = TempDir::new("iris_test").unwrap();
        let root = temp_dir.path();
        let config: PathBuf = root.join(".config/iris");
        let cache: PathBuf = root.join(".cache/iris");

        let paths = IrisPaths {
            config: config.clone(),
            cache: cache.clone(),
            generators: cache.join("gen"),
            bin: cache.join("bin"),
            state_file: config.join("state.toml"),
            current_theme: root.join(".cache/nvim/iris_current_theme"),
            themes: cache.join("themes"),
        };

        for dir in [
            &paths.config,
            &paths.themes,
            &paths.generators,
            &paths.bin,
            paths.current_theme.parent().unwrap(),
        ] {
            fs::create_dir_all(dir).unwrap();
        }

        let ctx = IrisContext {
            paths,
            state: State::default(),
            registry: GeneratorRegistry::default(),
            log: Logger::silent(),
            templater: Templater::mock(templates),
        };

        (temp_dir, ctx)
    }
}

/// Unit-tests for context
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::IrisContext;

    #[test]
    fn should_handle_context_update_theme_persistence() {
        let (_tmp, mut ctx) = IrisContext::mock();
        let theme_name = "melange";

        ctx.update(theme_name)
            .expect("Should update theme without errors");

        assert_eq!(ctx.state.theme.current_theme, theme_name.to_string());

        let state_content = fs::read_to_string(&ctx.paths.state_file).unwrap();
        assert!(state_content.contains(theme_name));

        let current_theme_content = fs::read_to_string(&ctx.paths.current_theme).unwrap();
        assert_eq!(current_theme_content, theme_name);
    }

    #[test]
    fn should_handle_context_save_state() {
        let (_tmp, mut ctx) = IrisContext::mock();
        ctx.state.enable_generator("yazi");
        ctx.save().expect("Should save state");

        let content = fs::read_to_string(&ctx.paths.state_file).unwrap();
        assert!(content.contains("yazi"));
    }

    #[test]
    fn should_handle_loading_context_from_existing_file() {
        let (_tmp, ctx_orig) = IrisContext::mock();
        let mut ctx = ctx_orig;
        ctx.update("gruvbox").unwrap();

        let state_toml = fs::read_to_string(&ctx.paths.state_file).unwrap();
        let loaded_state: State = toml::from_str(&state_toml).unwrap();

        assert_eq!(loaded_state.theme.current_theme, "gruvbox".to_string());
    }

    #[test]
    fn should_resolve_theme_explicit_exists() {
        let (_temp, ctx) = IrisContext::mock();

        fs::create_dir_all(&ctx.paths.themes).unwrap();
        let cache_path = ctx.paths.cached_theme("gruvbox");
        fs::write(&cache_path, "{}").unwrap();

        let (name, fallback) = ctx.resolve_theme(Some("Gruvbox".into()), false).unwrap();

        assert_eq!(name, "gruvbox");
        assert!(!fallback);
    }

    #[test]
    fn should_resolve_theme_fallback_to_current() {
        let (_temp, mut ctx) = IrisContext::mock();
        ctx.state.theme.current_theme = "nord".into();

        fs::create_dir_all(&ctx.paths.themes).unwrap();
        let cache_path = ctx.paths.cached_theme("nord");
        fs::write(&cache_path, "{}").unwrap();

        let (name, fallback) = ctx.resolve_theme(None, false).unwrap();

        assert_eq!(name, "nord");
        assert!(!fallback);
    }

    #[test]
    fn should_resolve_theme_use_fallback_theme_on_error() {
        let (_temp, mut ctx) = IrisContext::mock();
        ctx.state.theme.fallback_theme = "tokyonight".into();

        fs::create_dir_all(&ctx.paths.themes).unwrap();
        let cache_path = ctx.paths.cached_theme("tokyonight");
        fs::write(&cache_path, "{}").unwrap();

        let (name, fallback) = ctx.resolve_theme(Some("invalid".into()), true).unwrap();

        assert_eq!(name, "tokyonight");
        assert!(fallback);
    }

    #[test]
    fn should_check_if_theme_available_builtin() {
        let (_temp, mut ctx) = IrisContext::mock();
        ctx.state.nvim.manager = PluginManager::Default;

        assert!(ctx.is_theme_available("habamax"));
        assert!(!ctx.is_theme_available("non-existent-theme-123"));
    }

    #[test]
    fn should_handle_resolve_generator_not_found() {
        let (_temp, ctx) = IrisContext::mock();
        let result = ctx.resolve_generator("ghostty");
        assert!(result.is_err());

        let err_msg = format!("{:?}", result.err().unwrap());
        assert!(err_msg.contains("Unknown generator"));
    }
}
