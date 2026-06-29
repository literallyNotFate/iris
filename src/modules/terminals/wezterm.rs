use crate::{
    core::{IrisPaths, Templater},
    guards::FsRollbackGuard,
    log::Activity,
    models::{HealthStatus, Theme},
    modules::{Generator, GeneratorType},
    utils,
};
use anyhow::{Context, Result};
use colored::*;
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Config generator for wezterm terminal
pub struct WezTermGenerator;

impl Generator for WezTermGenerator {
    fn name(&self) -> &str {
        "wezterm"
    }

    fn generator_type(&self) -> GeneratorType {
        GeneratorType::Terminal
    }

    fn target_file_name(&self, theme: &str) -> String {
        if theme.is_empty() {
            "iris_theme.lua".into()
        } else {
            format!("{}.lua", theme.to_lowercase())
        }
    }

    fn link_path(&self, paths: &IrisPaths, _theme: &str) -> PathBuf {
        self.resolve_config_directory(paths)
            .join(self.target_file_name(""))
    }

    fn active_link_path(&self, paths: &IrisPaths) -> Option<PathBuf> {
        Some(self.resolve_config_directory(paths).join("iris_theme.lua"))
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
        c.insert("cursor", &theme.colors.sel);
        c.insert("ansi", &theme.colors.ansi);
        c
    }

    fn health_check(&self, paths: &IrisPaths, theme: &str) -> HealthStatus {
        if !self.is_installed() {
            return HealthStatus::Warning("`wezterm` binary not found".into());
        }

        let wezterm_dir: PathBuf = self.resolve_config_directory(paths);
        let config_path: PathBuf = wezterm_dir.join("wezterm.lua");
        let link_path: PathBuf = self.link_path(paths, "");

        let expected_cache: PathBuf = self.cache_path(paths, theme);
        let abs_expected_cache: PathBuf = if expected_cache.exists() {
            fs::canonicalize(&expected_cache).unwrap_or(expected_cache)
        } else {
            expected_cache
        };

        let config_status = HealthStatus::check_file(&config_path, "`wezterm` config file");
        if config_status.is_error() {
            return HealthStatus::error(
                "`wezterm.lua` main configuration file missing",
                Some(format!("Create {}", config_path.display())),
            );
        }

        let content: String = fs::read_to_string(&config_path).unwrap_or_default();
        if !content.contains("iris_theme") || !content.contains("config.colors") {
            return HealthStatus::Warning(format!(
                "Iris theme hook or color assignment missing in {}",
                config_path.display()
            ));
        }

        let symlink_status = HealthStatus::check_symlink(&link_path, "iris_theme.lua link");
        if symlink_status.is_error() {
            return HealthStatus::error(
                "iris_theme.lua missing or invalid",
                Some("Run `iris sync` or `iris health --fix` to recreate the link"),
            );
        }

        #[cfg(unix)]
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
        let wezterm_dir: PathBuf = self.resolve_config_directory(paths);
        let config_path: PathBuf = wezterm_dir.join("wezterm.lua");

        if status.contains("main configuration file missing")
            || status.contains("color assignment missing")
        {
            let config_backup: PathBuf = config_path.with_extension("bak");
            let rollback_guard = FsRollbackGuard::new(config_path.clone(), config_backup);

            let msg = if status.contains("main configuration file missing") {
                "Created missing `wezterm` config file with iris hook"
            } else {
                "Injected theme import hook into `wezterm.lua`"
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
                .action("Repaired `wezterm` theme files and symlinks", || {
                    self.ensure_cache_file(theme, paths, templater)?;
                    self.ensure_symlink(&cache, &link)
                })?;

            rollback_guard.commit();
            fixed = true;
        }

        if !fixed {
            task.log
                .action("Regenerated complete `wezterm` configuration", || {
                    self.apply(theme, paths, templater, &mut task.muted())
                })?;
        }

        Ok(())
    }
}

impl WezTermGenerator {
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
                format!("Failed to create `wezterm` directory: {}", parent.display())
            })?;
        }

        fs::write(&cache_file, content).with_context(|| {
            format!(
                "Failed to write `wezterm` cache file: {}",
                cache_file.display()
            )
        })?;
        Ok(cache_file)
    }

    fn ensure_symlink(&self, target: &Path, link: &Path) -> Result<()> {
        if let Some(parent) = link.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "Failed to create parent directory for `wezterm` link: {}",
                    parent.display()
                )
            })?;
        }

        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_micros())
            .unwrap_or(0);
        let tmp_link = link.with_extension(format!("tmp-{}", ts));

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            symlink(target, &tmp_link).with_context(|| {
                format!(
                    "Failed to create temporary `wezterm` symlink: {} -> {}",
                    target.display(),
                    tmp_link.display()
                )
            })?;
        }

        fs::rename(&tmp_link, link).with_context(|| {
            let _ = fs::remove_file(&tmp_link);
            format!(
                "Failed to atomically replace `wezterm` symlink at {}",
                link.display()
            )
        })?;

        Ok(())
    }

    fn inject_import_line(&self, paths: &IrisPaths) -> Result<()> {
        let config_path: PathBuf = self.resolve_config_directory(paths).join("wezterm.lua");
        if !config_path.exists() {
            if let Some(parent) = config_path.parent() {
                fs::create_dir_all(parent).with_context(|| {
                    format!(
                        "Failed to create parent `wezterm` directory: {}",
                        parent.display()
                    )
                })?;
            }

            let default_config = concat!(
                "local has_iris, iris_theme = pcall(require, \"iris_theme\")\n",
                "local wezterm = require(\"wezterm\")\n",
                "local config = wezterm.config_builder()\n\n",
                "if has_iris and iris_theme.colors then\n",
                "    config.colors = iris_theme.colors\n",
                "end\n\n",
                "return config\n"
            );

            fs::write(&config_path, default_config).with_context(|| {
                format!(
                    "Failed to create `wezterm` config file: {}",
                    config_path.display()
                )
            })?;
            return Ok(());
        }

        let content = fs::read_to_string(&config_path).with_context(|| {
            format!("Failed to read `wezterm` config: {}", config_path.display())
        })?;

        if content.contains("iris_theme") && content.contains("config.colors") {
            return Ok(());
        }

        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        if !content.contains("iris_theme") {
            lines.insert(
                0,
                "local has_iris, iris_theme = pcall(require, \"iris_theme\")".into(),
            );
        }

        if !content.contains("config.colors") {
            let mut insert_index = None;
            for (i, line) in lines.iter().enumerate() {
                if line.trim().starts_with("return ") {
                    insert_index = Some(i);
                    break;
                }
            }

            let inject_colors_block = concat!(
                "\n-- Iris Theme Manager Control Hook\n",
                "if has_iris and iris_theme.colors then\n",
                "    config.colors = iris_theme.colors\n",
                "end\n"
            );

            if let Some(index) = insert_index {
                lines.insert(index, inject_colors_block.into());
            } else {
                lines.push(inject_colors_block.into());
            }
        }

        fs::write(&config_path, lines.join("\n")).with_context(|| {
            format!("Failed to inject iris hook into: {}", config_path.display())
        })?;

        Ok(())
    }
}

/// Tests for wezterm generator
#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::tests::mock_context;

    /// Unit-tests for wezterm
    mod unit {
        use super::*;

        #[test]
        fn should_return_wezterm_metadata() {
            let generator = WezTermGenerator;
            assert_eq!(generator.name(), "wezterm");
            assert_eq!(generator.generator_type(), GeneratorType::Terminal);
            assert_eq!(generator.target_file_name("gruvbox"), "gruvbox.lua");
        }

        #[test]
        fn should_build_valid_render_context() {
            let generator = WezTermGenerator;
            let theme: Theme = Theme::mock();
            let ctx = generator.build_render_context(&theme);

            assert_eq!(ctx.get("bg").unwrap().as_str().unwrap(), theme.colors.bg);
            assert_eq!(ctx.get("fg").unwrap().as_str().unwrap(), theme.colors.fg);
            assert!(ctx.get("ansi").unwrap().is_array());
        }

        #[test]
        fn should_apply_theme_for_wezterm() {
            let (_, ctx) = mock_context();
            let generator = WezTermGenerator;
            let theme: Theme = Theme::mock();

            let mut task = ctx.log.step("Test", false).muted();
            let result = generator.apply(&theme, &ctx.paths, &ctx.templater, &mut task);
            assert!(result.is_ok(), "Failed to apply: {:?}", result.err());

            let cache_file = ctx.paths.generators.join("wezterm").join("test-theme.lua");
            assert!(cache_file.exists());

            let content = fs::read_to_string(cache_file).unwrap();

            assert!(content.contains("background ="));
            assert!(content.contains("foreground ="));
            assert!(content.contains("ansi ="));
        }

        #[test]
        fn should_clear_generated_files_for_wezterm() {
            let (_, ctx) = mock_context();
            let generator = WezTermGenerator;

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
        fn should_remove_theme_for_wezterm() {
            let (_, ctx) = mock_context();
            let generator = WezTermGenerator;

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

    /// Integration tests for wezterm
    mod integration {
        use super::*;
        use crate::skip_if_not_installed;

        #[test]
        fn should_return_health_ok_for_wezterm() {
            skip_if_not_installed!(WezTermGenerator);

            let (_, mut ctx) = mock_context();
            let generator = WezTermGenerator;
            let theme: Theme = Theme::mock();

            let mut task = ctx.log.step("Test", false).muted();
            ctx.state.theme.current_theme = theme.name.clone();
            generator
                .apply(&theme, &ctx.paths, &ctx.templater, &mut task)
                .unwrap();

            let config_path = generator
                .resolve_config_directory(&ctx.paths)
                .join("wezterm.lua");

            let valid_config = "local has_iris, iris_theme = pcall(require, 'iris_theme')\nconfig.colors = iris_theme.colors";
            fs::write(&config_path, valid_config).unwrap();

            let status = generator.health_check(&ctx.paths, &theme.name);
            assert!(status.is_ok(), "Expected Ok, got: {status}");
        }

        #[test]
        fn should_return_health_error_missing_config_for_wezterm() {
            skip_if_not_installed!(WezTermGenerator);

            let (_, ctx) = mock_context();
            let generator = WezTermGenerator;
            let theme: Theme = Theme::mock();

            let status = generator.health_check(&ctx.paths, &theme.name);
            assert!(status.is_error(), "Expected Error, got: {status}");
            assert!(status.contains("main configuration file missing"));
        }

        #[test]
        fn should_return_health_warning_no_import_for_wezterm() {
            skip_if_not_installed!(WezTermGenerator);

            let (_, mut ctx) = mock_context();
            let generator = WezTermGenerator;
            let theme: Theme = Theme::mock();

            let mut task = ctx.log.step("Test", false).muted();
            ctx.state.theme.current_theme = theme.name.clone();
            generator
                .apply(&theme, &ctx.paths, &ctx.templater, &mut task)
                .unwrap();

            let config_path = generator
                .resolve_config_directory(&ctx.paths)
                .join("wezterm.lua");
            fs::write(&config_path, "local config = wezterm.config_builder()").unwrap();

            let status = generator.health_check(&ctx.paths, &theme.name);

            assert!(
                status.is_warning(),
                "Expected Warning for missing import line, got: {status}"
            );
            assert!(status.contains("color assignment missing"));
        }

        #[test]
        fn should_fix_inject_issue_for_wezterm() {
            skip_if_not_installed!(WezTermGenerator);

            let (_, ctx) = mock_context();
            let generator = WezTermGenerator;
            let theme: Theme = Theme::mock();

            let config_dir = generator.resolve_config_directory(&ctx.paths);
            fs::create_dir_all(&config_dir).unwrap();
            let config_path = config_dir.join("wezterm.lua");

            let mut task = ctx.log.step("Test", false).muted();
            generator
                .apply(&theme, &ctx.paths, &ctx.templater, &mut task)
                .unwrap();

            fs::write(
                &config_path,
                "local wezterm = require(\"wezterm\")\nreturn config",
            )
            .unwrap();

            let status = generator.health_check(&ctx.paths, &theme.name);
            generator
                .fix(&status, &theme, &ctx.paths, &ctx.templater, &mut task)
                .expect("Fix failed");

            let content = fs::read_to_string(&config_path).unwrap();
            assert!(content.contains("iris_theme"));
            assert!(content.contains("config.colors = iris_theme.colors"));
            assert!(generator.health_check(&ctx.paths, &theme.name).is_ok());
        }

        #[test]
        fn should_fix_broken_link_for_wezterm() {
            skip_if_not_installed!(WezTermGenerator);

            let (_, mut ctx) = mock_context();
            let generator = WezTermGenerator;
            let theme: Theme = Theme::mock();
            ctx.state.theme.current_theme = theme.name.clone();

            let mut task = ctx.log.step("Test", false).muted();
            let wezterm_dir = generator.resolve_config_directory(&ctx.paths);
            let config_path = wezterm_dir.join("wezterm.lua");
            fs::create_dir_all(&wezterm_dir).unwrap();

            let valid_config = "local has_iris, iris_theme = pcall(require, 'iris_theme')\nconfig.colors = iris_theme.colors";
            fs::write(&config_path, valid_config).unwrap();

            generator
                .apply(&theme, &ctx.paths, &ctx.templater, &mut task)
                .unwrap();

            let link_path_empty = generator.link_path(&ctx.paths, "");
            if link_path_empty.exists() {
                fs::remove_file(&link_path_empty).unwrap();
            }

            let link_path_theme = generator.link_path(&ctx.paths, &theme.name);
            if link_path_theme.exists() && link_path_theme != link_path_empty {
                fs::remove_file(&link_path_theme).unwrap();
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
