use crate::{
    core::{IrisPaths, Templater},
    guards::FsRollbackGuard,
    log::Activity,
    models::{HealthStatus, Theme},
    modules::{Generator, GeneratorType},
    utils,
};
use anyhow::{Context, Result};
use colored::Colorize;
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Config generator for kitty terminal
pub struct KittyGenerator;

impl Generator for KittyGenerator {
    fn name(&self) -> &str {
        "kitty"
    }

    fn generator_type(&self) -> GeneratorType {
        GeneratorType::Terminal
    }

    fn target_file_name(&self, theme: &str) -> String {
        if theme.is_empty() {
            "current_theme.conf".into()
        } else {
            format!("{}.conf", theme.to_lowercase())
        }
    }

    fn link_path(&self, paths: &IrisPaths, _theme: &str) -> PathBuf {
        self.resolve_config_directory(paths)
            .join(self.target_file_name(""))
    }

    fn active_link_path(&self, paths: &IrisPaths) -> Option<PathBuf> {
        Some(
            self.resolve_config_directory(paths)
                .join("current_theme.conf"),
        )
    }

    fn apply(
        &self,
        theme: &Theme,
        paths: &IrisPaths,
        templater: &Templater,
        task: &mut Activity,
    ) -> Result<()> {
        task.info(&format!(
            "Generating {} theme for {}",
            theme.name.yellow(),
            self.name().bold().cyan(),
        ));

        let cache_file: PathBuf = self.ensure_cache_file(theme, paths, templater)?;
        let link_path: PathBuf = self.link_path(paths, &theme.name);
        let backup_path: PathBuf = link_path.with_extension("bak");

        task.info(&format!(
            "Linking {} theme to {}...",
            self.name().bold(),
            utils::pretty_path(&link_path).magenta(),
        ));

        let rollback_guard = FsRollbackGuard::new(link_path.clone(), backup_path);

        self.ensure_symlink(&cache_file, &link_path)?;
        rollback_guard.commit();

        task.info(&format!(
            "{} theme applied to {}",
            theme.name.yellow(),
            self.name().bold().cyan()
        ));
        Ok(())
    }

    fn build_render_context(&self, theme: &Theme) -> tera::Context {
        let mut c = tera::Context::new();
        c.insert("theme_name", &theme.name);
        c.insert("bg", &theme.colors.bg);
        c.insert("fg", &theme.colors.fg);
        c.insert("cursor", &theme.colors.caret);
        c.insert("sel_bg", &theme.colors.sel);
        c.insert("ansi", &theme.colors.ansi);
        c
    }

    fn health_check(&self, paths: &IrisPaths, theme: &str) -> HealthStatus {
        if !self.is_installed() {
            return HealthStatus::Warning("`kitty` binary not found".into());
        }

        let kitty_dir: PathBuf = self.resolve_config_directory(paths);
        let config_path: PathBuf = kitty_dir.join("kitty.conf");
        let link_path: PathBuf = self.link_path(paths, "");

        let expected_cache: PathBuf = self.cache_path(paths, &theme.to_lowercase());
        let abs_expected_cache: PathBuf = if expected_cache.exists() {
            fs::canonicalize(&expected_cache).unwrap_or(expected_cache)
        } else {
            expected_cache
        };

        let config_status = HealthStatus::check_file(&config_path, "`kitty` config file");
        if config_status.is_error() {
            return HealthStatus::error(
                "`kitty` config file missing",
                Some(format!("Create {}", config_path.display())),
            );
        }

        let content: String = fs::read_to_string(&config_path).unwrap_or_default();
        let import_line: String = format!("include {}", self.target_file_name(""));
        if !content.contains(&import_line) {
            return HealthStatus::Warning(format!(
                "Theme is generated but not imported in {}",
                config_path.display()
            ));
        }

        let symlink_status = HealthStatus::check_symlink(&link_path, "current_theme.conf link");
        if symlink_status.is_error() {
            return HealthStatus::error(
                "current_theme.conf missing or invalid",
                Some("Run `iris sync` or `iris health --fix` to recreate the link"),
            );
        }

        if let Ok(target) = fs::read_link(&link_path) {
            let abs_target: PathBuf = fs::canonicalize(&target).unwrap_or(target);
            if abs_target != abs_expected_cache {
                return HealthStatus::Warning("Link points to a different cache location".into());
            }
        }

        HealthStatus::Ok
    }

    fn fix(
        &self,
        status: &HealthStatus,
        theme: &Theme,
        paths: &IrisPaths,
        templater: &Templater,
        task: &mut Activity,
    ) -> Result<()> {
        if !status.is_error() && !status.is_warning() {
            return task.log.action(
                &format!("Re-applied `{}` configuration", self.name().bold()),
                || self.apply(theme, paths, templater, &mut task.muted()),
            );
        }

        let mut fixed = false;
        let kitty_dir: PathBuf = self.resolve_config_directory(paths);
        let config_path: PathBuf = kitty_dir.join("kitty.conf");

        if status.contains("config file missing") || status.contains("not imported") {
            let config_backup: PathBuf = config_path.with_extension("bak");
            let rollback_guard = FsRollbackGuard::new(config_path.clone(), config_backup);

            let msg: &str = if status.contains("config file missing") {
                "Created missing `kitty` config file"
            } else {
                "Injected theme include line into `kitty` config"
            };

            task.log.action(msg, || self.inject_import_line(paths))?;

            rollback_guard.commit();
            fixed = true;
        }

        if status.contains("missing or invalid") || status.contains("different cache") {
            let link: PathBuf = self.link_path(paths, "");
            let backup: PathBuf = link.with_extension("bak");
            let cache: PathBuf = self.cache_path(paths, &theme.name.to_lowercase());

            let rollback_guard = FsRollbackGuard::new(link.clone(), backup);

            task.log
                .action("Repaired `kitty` theme files and symlinks", || {
                    self.ensure_cache_file(theme, paths, templater)?;
                    self.ensure_symlink(&cache, &link)
                })?;

            rollback_guard.commit();
            fixed = true;
        }

        if !fixed {
            task.log
                .action("Regenerated complete `kitty` configuration", || {
                    self.apply(theme, paths, templater, &mut task.muted())
                })?;
        }

        Ok(())
    }
}

impl KittyGenerator {
    fn ensure_cache_file(
        &self,
        theme: &Theme,
        paths: &IrisPaths,
        templater: &Templater,
    ) -> Result<PathBuf> {
        let cache_file: PathBuf = self.cache_path(paths, &theme.name.to_lowercase());
        let render_ctx = self.build_render_context(theme);
        let content: String = templater.render(&self.template_path(), &render_ctx)?;

        if let Some(parent) = cache_file.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create `kitty` directory: {}", parent.display())
            })?;
        }

        fs::write(&cache_file, content).with_context(|| {
            format!(
                "Failed to write `kitty` cache file: {}",
                cache_file.display()
            )
        })?;
        Ok(cache_file)
    }

    fn ensure_symlink(&self, target: &Path, link: &Path) -> Result<()> {
        if link.exists() || link.is_symlink() {
            fs::remove_file(link).with_context(|| {
                format!(
                    "Failed to remove existing `kitty` file/link at {}",
                    link.display()
                )
            })?;
        }

        if let Some(parent) = link.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "Failed to create parent directory for `kitty` link: {}",
                    parent.display()
                )
            })?;
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            symlink(target, link).with_context(|| {
                format!(
                    "Failed to create `kitty` symlink: {} -> {}",
                    target.display(),
                    link.display()
                )
            })?;
        }
        Ok(())
    }

    fn inject_import_line(&self, paths: &IrisPaths) -> Result<()> {
        let config_path: PathBuf = self.resolve_config_directory(paths).join("kitty.conf");
        let import_line: String = format!("include {}", self.target_file_name(""));

        if !config_path.exists() {
            if let Some(parent) = config_path.parent() {
                fs::create_dir_all(parent).with_context(|| {
                    format!(
                        "Failed to create parent `kitty` directory: {}",
                        parent.display()
                    )
                })?;
            }

            fs::write(&config_path, format!("{}\n", import_line)).with_context(|| {
                format!(
                    "Failed to create `kitty` config file: {}",
                    config_path.display()
                )
            })?;
            return Ok(());
        }

        let content = fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read `kitty` config: {}", config_path.display()))?;

        if !content.contains(&import_line) {
            use std::fs::OpenOptions;
            use std::io::Write;

            let mut file = OpenOptions::new()
                .append(true)
                .open(&config_path)
                .with_context(|| {
                    format!(
                        "Failed to open `kitty` config for appending: {}",
                        config_path.display()
                    )
                })?;

            writeln!(file, "\n{}", import_line).with_context(|| {
                format!(
                    "Failed to write to `kitty` config: {}",
                    config_path.display()
                )
            })?;
        }

        Ok(())
    }
}

/// Tests for kitty generator
#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::tests::mock_context;

    /// Unit-tests for kitty
    mod unit {
        use super::*;

        #[test]
        fn should_return_kitty_metadata() {
            let generator = KittyGenerator;
            assert_eq!(generator.name(), "kitty");
            assert_eq!(generator.generator_type(), GeneratorType::Terminal);
            assert_eq!(generator.target_file_name("melange"), "melange.conf");
            assert_eq!(generator.target_file_name(""), "current_theme.conf");
        }

        #[test]
        fn should_build_valid_render_context() {
            let generator = KittyGenerator;
            let theme: Theme = Theme::mock();
            let ctx = generator.build_render_context(&theme);

            assert_eq!(ctx.get("bg").unwrap().as_str().unwrap(), theme.colors.bg);
            assert_eq!(
                ctx.get("sel_bg").unwrap().as_str().unwrap(),
                theme.colors.sel
            );
            assert!(ctx.get("ansi").unwrap().is_array());
        }

        #[test]
        fn should_apply_theme_for_kitty() {
            let (_, ctx) = mock_context();
            let generator = KittyGenerator;
            let theme: Theme = Theme::mock();

            let mut task = ctx.log.step("Test", false).muted();
            let result = generator.apply(&theme, &ctx.paths, &ctx.templater, &mut task);
            assert!(result.is_ok(), "Failed to apply: {:?}", result.err());

            let cache_file = ctx.paths.generators.join("kitty").join("test-theme.conf");
            assert!(cache_file.exists());

            let content = fs::read_to_string(cache_file).unwrap();
            assert!(content.contains("background"));
            assert!(content.contains("color0"));
            assert!(content.contains("selection_foreground none"));
        }

        #[test]
        fn should_clear_generated_files_for_kitty() {
            let (_, ctx) = mock_context();
            let generator = KittyGenerator;

            let cache_dir = ctx.paths.generators.join(generator.name());
            fs::create_dir_all(&cache_dir).unwrap();
            let file = cache_dir.join(generator.target_file_name(""));
            fs::write(&file, "test").unwrap();

            assert!(
                cache_dir.exists(),
                "Cache directory should exist before clearing"
            );

            generator.clear(&ctx.paths).unwrap();

            assert!(
                !cache_dir.exists(),
                "Clear should remove the entire generator cache directory"
            );
        }

        #[test]
        fn should_remove_theme_for_kitty() {
            let (_, ctx) = mock_context();
            let generator = KittyGenerator;

            let cache_file = generator.cache_path(&ctx.paths, "");
            let link_file = generator.link_path(&ctx.paths, "");

            fs::create_dir_all(cache_file.parent().unwrap()).unwrap();
            fs::create_dir_all(link_file.parent().unwrap()).unwrap();
            fs::write(&cache_file, "test").unwrap();
            fs::write(&link_file, "test").unwrap();

            assert!(
                cache_file.exists(),
                "Cache file should exist before removal"
            );
            assert!(link_file.exists(), "Link file should exist before removal");

            generator.remove_theme(&ctx.paths, "").unwrap();

            assert!(
                !cache_file.exists(),
                "remove_theme should delete the cache file"
            );
            assert!(
                !link_file.exists(),
                "remove_theme should delete the target link file"
            );
        }
    }

    /// Integration tests for kitty
    mod integration {
        use super::*;
        use crate::skip_if_not_installed;

        #[test]
        fn should_return_health_ok_for_kitty() {
            skip_if_not_installed!(KittyGenerator);

            let (_, mut ctx) = mock_context();
            let generator = KittyGenerator;
            let theme: Theme = Theme::mock();

            let mut task = ctx.log.step("Test", false).muted();
            ctx.state.theme.current_theme = theme.name.clone();
            generator
                .apply(&theme, &ctx.paths, &ctx.templater, &mut task)
                .unwrap();

            let config_path = generator
                .resolve_config_directory(&ctx.paths)
                .join("kitty.conf");

            let content = format!("include {}", generator.target_file_name(""));
            fs::write(&config_path, content).unwrap();

            let status = generator.health_check(&ctx.paths, &theme.name);
            assert!(status.is_ok(), "Expected Ok, got: {status}");
        }

        #[test]
        fn should_return_health_error_missing_config_for_kitty() {
            skip_if_not_installed!(KittyGenerator);

            let (_, ctx) = mock_context();
            let generator = KittyGenerator;
            let theme: Theme = Theme::mock();

            let status = generator.health_check(&ctx.paths, &theme.name);
            assert!(status.is_error(), "Expected Error, got: {status}");
            assert!(status.contains("config file missing"));
        }

        #[test]
        fn should_return_health_warning_no_import_for_kitty() {
            skip_if_not_installed!(KittyGenerator);

            let (_, mut ctx) = mock_context();
            let generator = KittyGenerator;
            let theme: Theme = Theme::mock();

            let mut task = ctx.log.step("Test", false).muted();
            ctx.state.theme.current_theme = theme.name.clone();
            generator
                .apply(&theme, &ctx.paths, &ctx.templater, &mut task)
                .unwrap();

            let config_path = generator
                .resolve_config_directory(&ctx.paths)
                .join("kitty.conf");
            fs::create_dir_all(config_path.parent().unwrap()).unwrap();
            fs::write(&config_path, "font_size 18").unwrap();

            let status = generator.health_check(&ctx.paths, &theme.name);

            assert!(
                status.is_warning(),
                "Expected Warning for missing import line, got: {status}"
            );
            assert!(status.contains("Theme is generated but not imported"));
        }

        #[test]
        fn should_fix_inject_issue_and_remote_control_for_kitty() {
            skip_if_not_installed!(KittyGenerator);

            let (_, ctx) = mock_context();
            let generator = KittyGenerator;
            let theme: Theme = Theme::mock();

            let config_dir = generator.resolve_config_directory(&ctx.paths);
            fs::create_dir_all(&config_dir).unwrap();
            let config_path = config_dir.join("kitty.conf");

            let mut task = ctx.log.step("Test", false).muted();
            generator
                .apply(&theme, &ctx.paths, &ctx.templater, &mut task)
                .unwrap();
            fs::write(&config_path, "font_size 12").unwrap();

            let status = generator.health_check(&ctx.paths, &theme.name);
            generator
                .fix(&status, &theme, &ctx.paths, &ctx.templater, &mut task)
                .expect("First fix failed");

            let status = generator.health_check(&ctx.paths, &theme.name);
            generator
                .fix(&status, &theme, &ctx.paths, &ctx.templater, &mut task)
                .expect("Second fix failed");

            let content = fs::read_to_string(&config_path).unwrap();
            assert!(content.contains("current_theme.conf"));
            assert!(generator.health_check(&ctx.paths, &theme.name).is_ok());
        }

        #[test]
        fn should_fix_broken_link_for_kitty() {
            skip_if_not_installed!(KittyGenerator);

            let (_, mut ctx) = mock_context();
            let generator = KittyGenerator;
            let theme: Theme = Theme::mock();
            ctx.state.theme.current_theme = theme.name.clone();

            let mut task = ctx.log.step("Test", false).muted();
            let kitty_dir = generator.resolve_config_directory(&ctx.paths);
            let config_path = kitty_dir.join("kitty.conf");
            fs::create_dir_all(&kitty_dir).unwrap();

            let content = format!("include {}", generator.target_file_name(""));
            fs::write(&config_path, content).unwrap();

            generator
                .apply(&theme, &ctx.paths, &ctx.templater, &mut task)
                .unwrap();

            let link_path_empty = generator.link_path(&ctx.paths, "");
            if link_path_empty.exists() {
                fs::remove_file(&link_path_empty).unwrap();
            }

            let status = generator.health_check(&ctx.paths, &theme.name);
            assert!(status.is_error());
            assert!(status.contains("missing or invalid"));

            generator
                .fix(&status, &theme, &ctx.paths, &ctx.templater, &mut task)
                .unwrap();
            assert!(generator.health_check(&ctx.paths, &theme.name).is_ok());
        }
    }
}
