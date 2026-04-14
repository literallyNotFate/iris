use crate::{commands::HealthStatus, core::IrisContext, models::Palette, modules::GeneratorType};
use std::{env, path::PathBuf};

/// Main trait for all generators
pub trait Generator: Send + Sync {
    /// Returns name of the generator (e.g "ghostty")
    fn name(&self) -> &str;

    /// Returns type of the generator
    fn generator_type(&self) -> GeneratorType;

    /// Returns the name of the file responsible for configuring app
    fn target_file_name(&self, theme: &str) -> String;

    /// Generator path in cache.
    /// By default: ~/.cache/iris/[name]/[target_file]
    fn cache_path(&self, ctx: &IrisContext, theme_name: &str) -> PathBuf {
        ctx.paths
            .cache
            .join(self.name())
            .join(self.target_file_name(theme_name))
    }

    /// Path, where app expects to apply config/theme
    /// By default: ~/.config/[name]/[target_file]
    fn link_path(&self, theme_name: &str) -> PathBuf {
        self.resolve_config_directory()
            .join(self.target_file_name(theme_name))
    }

    /// Dynamically forms ID template for Tera
    /// E.g. "tool/yazi", "terminal/alacritty"
    fn template_path(&self) -> String {
        format!("{}/{}", self.generator_type().label(), self.name())
    }

    /// Checks whether this tool is installed
    fn is_installed(&self) -> bool {
        which::which(self.name()).is_ok()
    }

    /// Logic of applying the theme (file writing, building cache etc)
    fn apply(&self, p: &Palette, ctx: &IrisContext) -> anyhow::Result<()>;

    /// Automatic config directory resolver
    fn resolve_config_directory(&self) -> PathBuf {
        if let Some(p) = self.env_config_directory() {
            if p.is_file() {
                return p.parent().unwrap_or(&p).to_path_buf();
            }

            return p;
        }

        if let Some(home) = dirs::home_dir() {
            let config_path = home.join(".config").join(self.name());
            return config_path;
        }

        let home = env::var("HOME").unwrap_or_else(|_| ".".into());
        PathBuf::from(home).join(".config").join(self.name())
    }

    /// Allows module to pass environment value path of the config (e.g STARSHIP_CONFIG)
    fn env_config_directory(&self) -> Option<PathBuf> {
        None
    }

    /// "Health" generator check
    /// Implementation by default checks if binary is installed
    fn health_check(&self, _ctx: &IrisContext) -> HealthStatus {
        HealthStatus::Ok
    }

    /// Basic template context builder
    /// Basically passes all palette colors to templater
    fn build_render_context(&self, p: &Palette) -> tera::Context;
}
