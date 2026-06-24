use crate::{
    core::{IrisPaths, Templater},
    log::Activity,
    models::{HealthStatus, Theme},
    modules::GeneratorType,
};
use anyhow::{Context, Result};
use colored::Colorize;
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
    fn cache_path(&self, paths: &IrisPaths, theme: &str) -> PathBuf {
        paths
            .generators
            .join(self.name())
            .join(self.target_file_name(theme))
    }

    /// Path, where app expects to apply config/theme.
    /// By default: ~/.config/[name]/[target_file]
    fn link_path(&self, paths: &IrisPaths, theme: &str) -> PathBuf {
        self.resolve_config_directory(paths)
            .join(self.target_file_name(theme))
    }

    /// Returns path to the static active link if app uses it,
    /// By default: None (means that app uses dynamic theme files like btop etc.)
    fn active_link_path(&self, _paths: &IrisPaths) -> Option<PathBuf> {
        None
    }

    /// Dynamically forms ID template for Tera.
    /// E.g. "tool/yazi", "terminal/alacritty"
    fn template_path(&self) -> String {
        format!("{}/{}", self.generator_type().label(), self.name())
    }

    /// Path to the theme file (either in cache or link)
    fn theme_path(&self, paths: &IrisPaths, theme: &str) -> PathBuf {
        self.link_path(paths, theme)
    }

    /// Checks whether this tool is installed
    fn is_installed(&self) -> bool {
        which::which(self.name()).is_ok()
    }

    /// Logic of applying the theme (file writing, building cache etc)
    fn apply(
        &self,
        theme: &Theme,
        paths: &IrisPaths,
        templater: &Templater,
        task: &mut Activity,
    ) -> Result<()>;

    /// Automatic config directory resolver
    fn resolve_config_directory(&self, paths: &IrisPaths) -> PathBuf {
        if let Some(p) = self.env_config_directory() {
            return if p.is_file() {
                p.parent().unwrap_or(&p).to_path_buf()
            } else {
                p
            };
        }

        let config_base: PathBuf = paths
            .config
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| paths.config.clone());

        config_base.join(self.name())
    }

    /// Allows module to pass environment value path of the config (e.g STARSHIP_CONFIG)
    fn env_config_directory(&self) -> Option<PathBuf> {
        None
    }

    /// "Health" generator check.
    /// Implementation by default checks if binary is installed
    fn health_check(&self, _paths: &IrisPaths, _theme: &str) -> HealthStatus {
        HealthStatus::Ok
    }

    /// Automatically fix detected issues (based on HealthStatus)
    fn fix(
        &self,
        status: &HealthStatus,
        theme: &Theme,
        paths: &IrisPaths,
        templater: &Templater,
        task: &mut Activity,
    ) -> Result<()>;

    /// Basic template context builder.
    /// Basically passes all palette colors to templater
    fn build_render_context(&self, theme: &Theme) -> tera::Context;

    /// Clear generator cached files
    fn clear(&self, paths: &IrisPaths) -> Result<()> {
        let name: &str = self.name();

        if let Some(active_link) = self.active_link_path(paths) {
            if active_link.exists() || active_link.is_symlink() {
                fs::remove_file(&active_link).with_context(|| {
                    format!(
                        "Failed to remove active link for {}: {}",
                        name.bold(),
                        active_link.display()
                    )
                })?;
            }
        }

        let static_link: PathBuf = self.link_path(paths, "");
        if static_link.is_symlink() || static_link.exists() {
            let _ = fs::remove_file(&static_link);
        }

        let app_config_dir = self.resolve_config_directory(paths);
        if app_config_dir.exists() && app_config_dir.is_dir() {
            if app_config_dir.file_name().map_or(false, |n| n == "themes") {
                let _ = fs::remove_dir_all(&app_config_dir);
            }
        }

        let gen_cache_dir: PathBuf = paths.generators.join(name);
        if gen_cache_dir.exists() {
            fs::remove_dir_all(&gen_cache_dir).with_context(|| {
                format!(
                    "Failed to remove generator directory for {}: {}",
                    name.bold(),
                    gen_cache_dir.display()
                )
            })?;
        }

        Ok(())
    }

    /// Removes cached files for generator of a certain theme
    fn remove_theme(&self, paths: &IrisPaths, theme_name: &str) -> Result<()> {
        let name: &str = self.name();
        let theme_name_lower: String = theme_name.to_lowercase();

        let cache_file: PathBuf = self.cache_path(paths, &theme_name_lower);
        let abs_cache_file = if cache_file.exists() {
            fs::canonicalize(&cache_file).unwrap_or(cache_file.clone())
        } else {
            cache_file.clone()
        };

        let static_theme_file: PathBuf = self.link_path(paths, "");
        let custom_theme_file: PathBuf = self.link_path(paths, &theme_name_lower);

        let mut targets = vec![static_theme_file, custom_theme_file];
        targets.dedup();

        for theme_file in targets {
            if theme_file.exists() || theme_file.is_symlink() {
                if theme_file.is_symlink() {
                    if let Ok(resolved_target) = fs::canonicalize(&theme_file) {
                        if resolved_target == abs_cache_file {
                            fs::remove_file(&theme_file).with_context(|| {
                                format!(
                                    "Failed to remove active symlink for {}: {}",
                                    name.bold(),
                                    theme_file.display()
                                )
                            })?;
                        }
                    }
                } else {
                    if theme_file != self.resolve_config_directory(paths) {
                        fs::remove_file(&theme_file).with_context(|| {
                            format!(
                                "Failed to remove theme file for {}: {}",
                                name.bold(),
                                theme_file.display()
                            )
                        })?;
                    }
                }
            }
        }

        if cache_file.exists() {
            fs::remove_file(&cache_file).with_context(|| {
                format!(
                    "Failed to remove {} cache file: {}",
                    name.bold(),
                    cache_file.display()
                )
            })?;
        }

        Ok(())
    }
}
