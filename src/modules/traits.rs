use crate::{commands::HealthStatus, core::IrisContext, models::Palette, modules::GeneratorType};
use std::{fs, path::PathBuf};

/// Main trait for all generators
pub trait Generator: Send + Sync {
    /// Returns name of the generator (e.g "ghostty")
    fn name(&self) -> &str;

    /// Returns type of the generator
    fn generator_type(&self) -> GeneratorType;

    /// Returns the name of the file responsible for configuring app
    fn target_file_name(&self, theme: &str) -> String;

    /// Generator path in cache.
    /// By default: ~/.cache/iris/gen/[name]/[target_file]
    fn cache_path(&self, ctx: &IrisContext, theme_name: &str) -> PathBuf {
        ctx.paths
            .generators
            .join(self.name())
            .join(self.target_file_name(theme_name))
    }

    /// Path, where app expects to apply config/theme
    /// By default: ~/.config/[name]/[target_file]
    fn link_path(&self, ctx: &IrisContext, theme_name: &str) -> PathBuf {
        self.resolve_config_directory(ctx)
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
    fn resolve_config_directory(&self, ctx: &IrisContext) -> PathBuf {
        if let Some(p) = self.env_config_directory() {
            return if p.is_file() {
                p.parent().unwrap_or(&p).to_path_buf()
            } else {
                p
            };
        }

        let config_base: PathBuf = ctx
            .paths
            .config
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| ctx.paths.config.clone());

        config_base.join(self.name())
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

    /// Automatically fix detected issues (based on HealthStatus)
    fn fix(&self, status: &HealthStatus, p: &Palette, ctx: &IrisContext) -> anyhow::Result<()>;

    /// Basic template context builder
    /// Basically passes all palette colors to templater
    fn build_render_context(&self, p: &Palette) -> tera::Context;

    /// Clear generator cached files
    fn clear(&self, ctx: &IrisContext) -> anyhow::Result<()> {
        let name: &str = self.name();
        let gen_cache_dir: PathBuf = ctx.paths.generators.join(name);
        if gen_cache_dir.exists() {
            fs::remove_dir_all(&gen_cache_dir)?;
        }

        let bin_file: PathBuf = self.cache_path(ctx, "");
        if bin_file.exists() {
            fs::remove_file(bin_file)?;
        }

        Ok(())
    }
}
