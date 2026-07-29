use crate::{infra::IrisPaths, modules::Generator};
use anyhow::{Context, Result};
use colored::Colorize;
use std::{fs, path::PathBuf};

pub trait Cleanable: Generator {
    /// Clear generator cached files
    fn cleanup(&self, paths: &IrisPaths) -> anyhow::Result<()>;

    /// Removes cached files for generator of a certain theme
    fn remove_theme(&self, paths: &IrisPaths, theme_name: &str) -> anyhow::Result<()>;

    /// Cleanup hook that clears config file
    fn cleanup_config(&self, _config_path: &PathBuf) -> anyhow::Result<()> {
        Ok(())
    }

    /// Cleanup hook that is being called before cache directory removal
    fn pre_cleanup(&self, _paths: &IrisPaths) -> anyhow::Result<()> {
        Ok(())
    }

    /// Cleanup hook that is called at the very end of cleanup
    fn post_cleanup(&self, _paths: &IrisPaths) -> anyhow::Result<()> {
        Ok(())
    }

    /// Cleanup hook that is called at the end of theme removal
    fn post_remove(&self, _paths: &IrisPaths, _theme_name: &str) -> anyhow::Result<()> {
        Ok(())
    }
}

/// Default cleanup logic implementation
pub fn default_cleanup<T: Cleanable + ?Sized>(g: &T, paths: &IrisPaths) -> anyhow::Result<()> {
    let config_path: PathBuf = g.link_path(paths, "");
    g.cleanup_config(&config_path)?;
    g.pre_cleanup(paths)?;

    let name: &str = g.name();
    if let Some(active_link) = g.active_link_path(paths) {
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

    let static_link: PathBuf = g.link_path(paths, "");
    if static_link.is_symlink() || static_link.exists() {
        let _ = fs::remove_file(&static_link).with_context(|| {
            format!(
                "Failed to remove static link for {}: {}",
                name.bold(),
                static_link.display()
            )
        })?;
    }

    let app_config_dir: PathBuf = g.resolve_config_directory(paths);
    if app_config_dir.exists() && app_config_dir.is_dir() {
        if app_config_dir.file_name().map_or(false, |n| n == "themes") {
            let _ = fs::remove_dir_all(&app_config_dir)
                .with_context(|| format!("Failed to remove themes/ folder for {}", name.bold()))?;
        }
    }

    let gen_cache_dir: PathBuf = paths.generators.join(name);
    if gen_cache_dir.exists() {
        fs::remove_dir_all(&gen_cache_dir).with_context(|| {
            format!(
                "Failed to remove generator cache directory for {}: {}",
                name.bold(),
                gen_cache_dir.display()
            )
        })?;
    }

    g.post_cleanup(paths)?;

    Ok(())
}

/// Default remove theme logic implementation
pub fn default_remove<T: Cleanable + ?Sized>(
    g: &T,
    paths: &IrisPaths,
    theme_name: &str,
) -> Result<()> {
    let name: &str = g.name();
    let theme_name_lower: String = theme_name.to_lowercase();

    let cache_file: PathBuf = g.cache_path(paths, &theme_name_lower);
    let abs_cache_file = if cache_file.exists() {
        fs::canonicalize(&cache_file).unwrap_or(cache_file.clone())
    } else {
        cache_file.clone()
    };

    let static_theme_file: PathBuf = g.link_path(paths, "");
    let custom_theme_file: PathBuf = g.link_path(paths, &theme_name_lower);

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
                if theme_file != g.resolve_config_directory(paths) {
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

    g.post_remove(paths, theme_name)?;

    Ok(())
}
