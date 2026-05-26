use crate::{
    core::{IrisPaths, Templater},
    log::Task,
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

/// Config generator for yazi
pub struct YaziGenerator;

impl Generator for YaziGenerator {
    fn name(&self) -> &str {
        "yazi"
    }

    fn generator_type(&self) -> GeneratorType {
        GeneratorType::Tool
    }

    fn target_file_name(&self, _theme: &str) -> String {
        "theme.toml".into()
    }

    fn link_path(&self, paths: &IrisPaths, _theme: &str) -> PathBuf {
        self.resolve_config_directory(paths)
            .join(self.target_file_name(""))
    }

    fn apply(
        &self,
        theme: &Theme,
        paths: &IrisPaths,
        templater: &Templater,
        task: &mut Task,
    ) -> Result<()> {
        let cache_file: PathBuf = self.ensure_cache_file(theme, paths, templater)?;
        let link_path: PathBuf = self.link_path(paths, &theme.name.to_lowercase());

        task.info(&format!(
            "Linking {} theme to {}...",
            self.name().bold().cyan(),
            utils::pretty_path(&link_path).magenta(),
        ));
        self.ensure_symlink(&cache_file, &link_path)?;

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
        c.insert("white", &theme.colors.white);
        c.insert("comment", &theme.colors.comment);
        c.insert("gutter_fg", &theme.colors.gutter_fg);
        c.insert("ansi", &theme.colors.ansi);
        c.insert("sel", &theme.colors.sel);

        let line_hl = if theme.colors.line_hl == "#cccccc" {
            &theme.colors.sel
        } else {
            &theme.colors.line_hl
        };
        c.insert("line_hl", line_hl);

        c.insert("red", &theme.colors.ansi[1]);
        c.insert("green", &theme.colors.ansi[2]);
        c.insert("orange", &theme.colors.ansi[3]);
        c.insert("blue", &theme.colors.ansi[4]);
        c.insert("magenta", &theme.colors.ansi[5]);
        c.insert("teal", &theme.colors.ansi[6]);
        c.insert("tan", &theme.colors.ansi[7]);
        c.insert("br_red", &theme.colors.ansi[9]);
        c.insert("br_green", &theme.colors.ansi[10]);
        c.insert("br_orange", &theme.colors.ansi[11]);
        c.insert("br_blue", &theme.colors.ansi[12]);
        c.insert("br_magenta", &theme.colors.ansi[13]);
        c.insert("br_teal", &theme.colors.ansi[14]);

        c
    }

    fn health_check(&self, paths: &IrisPaths, _theme: &str) -> HealthStatus {
        if !self.is_installed() {
            return HealthStatus::Warning("`yazi` binary not found".into());
        }

        let link_path: PathBuf = self.link_path(paths, "");
        let expected_cache: PathBuf = self.cache_path(paths, "");
        let cache_status = HealthStatus::check_file(&expected_cache, "`yazi` theme cache file");

        if cache_status.is_error() {
            return HealthStatus::error(
                "`yazi` theme cache file is missing",
                Some("run `iris sync` or `iris health --fix` to regenerate the cache"),
            );
        }

        let symlink_status =
            HealthStatus::check_symlink(&link_path, "theme.toml link in yazi config");
        if symlink_status.is_error() {
            return HealthStatus::error(
                "theme.toml link missing or invalid in yazi config",
                Some("run `iris sync` or `iris health --fix` to create the symlink"),
            );
        }

        #[cfg(unix)]
        if let Ok(target) = fs::read_link(&link_path) {
            if target != expected_cache {
                return HealthStatus::Warning(format!(
                    "`yazi` theme link points to an unexpected location: {:?}",
                    target
                ));
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
        task: &mut Task,
    ) -> Result<()> {
        if !status.is_error() && !status.is_warning() {
            return task.log.action(
                &format!("Re-applied `{}` configuration", self.name().bold()),
                || self.apply(theme, paths, templater, &mut task.as_quiet()),
            );
        }

        let mut fixed = false;
        if status.contains("cache file is missing") {
            task.log.action("Generated missing cache file", || {
                self.ensure_cache_file(theme, paths, templater)
            })?;
            fixed = true;
        }

        if status.contains("link missing") || status.contains("unexpected location") {
            task.log.action("Restored correct theme symlink", || {
                let cache = self.cache_path(paths, &theme.name.to_lowercase());
                let link = self.link_path(paths, &theme.name.to_lowercase());
                self.ensure_symlink(&cache, &link)
            })?;
            fixed = true;
        }

        if !fixed {
            task.log
                .action("Regenerated complete `yazi` configuration", || {
                    self.apply(theme, paths, templater, &mut task.as_quiet())
                })?;
        }

        Ok(())
    }
}

impl YaziGenerator {
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
                format!(
                    "Failed to create `yazi` cache directory: {}",
                    parent.display()
                )
            })?;
        }

        fs::write(&cache_file, content).with_context(|| {
            format!(
                "Failed to write `yazi` cache file: {}",
                cache_file.display()
            )
        })?;

        Ok(cache_file)
    }

    fn ensure_symlink(&self, target: &Path, link: &Path) -> Result<()> {
        if link.exists() || link.is_symlink() {
            fs::remove_file(link).with_context(|| {
                format!(
                    "Failed to remove `yazi` old symlink/file: {}",
                    link.display()
                )
            })?;
        }

        if let Some(parent) = link.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "Failed to create directory for `yazi` symlink: {}",
                    parent.display()
                )
            })?;
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            symlink(target, link).with_context(|| {
                format!(
                    "Failed to create `yazi` symlink: {} -> {}",
                    target.display(),
                    link.display()
                )
            })?;
        }

        Ok(())
    }
}

/// Unit-tests for yazi generator
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::tests::create_test_context;

    #[test]
    fn should_return_yazi_metadata() {
        let generator = YaziGenerator;
        assert_eq!(generator.name(), "yazi");
        assert_eq!(generator.generator_type(), GeneratorType::Tool);
        assert_eq!(generator.target_file_name("any"), "theme.toml");
    }

    #[test]
    fn should_build_valid_render_context() {
        let generator = YaziGenerator;
        let mut theme: Theme = Theme::mock();

        theme.colors.line_hl = "#123456".to_string();
        let ctx = generator.build_render_context(&theme);
        assert_eq!(ctx.get("line_hl").unwrap().as_str().unwrap(), "#123456");

        theme.colors.line_hl = "#cccccc".to_string();
        theme.colors.sel = "#ff0000".to_string();
        let ctx = generator.build_render_context(&theme);

        assert_eq!(ctx.get("line_hl").unwrap().as_str().unwrap(), "#ff0000");
        assert!(ctx.get("red").is_some());
        assert!(ctx.get("br_teal").is_some());
    }

    #[test]
    fn should_return_health_ok_for_yazi() {
        let (_, mut ctx) = create_test_context();
        let generator = YaziGenerator;
        let theme: Theme = Theme::mock();

        ctx.state.current_theme = theme.name.clone();
        let mut task = ctx.log.step("Test", false);
        generator
            .apply(&theme, &ctx.paths, &ctx.templater, &mut task)
            .unwrap();

        let status = generator.health_check(&ctx.paths, &theme.name);
        assert!(status.is_ok(), "Expected Ok, got: {status}");
    }

    #[test]
    fn should_return_health_error_missing_link_for_yazi() {
        let (_, ctx) = create_test_context();
        let generator = YaziGenerator;
        let link = generator.link_path(&ctx.paths, "");

        if link.exists() || link.is_symlink() {
            let _ = fs::remove_file(&link);
        }

        let status = generator.health_check(&ctx.paths, &ctx.state.current_theme);
        assert!(status.is_error(), "Expected Error, got: {status}");
        assert!(status.contains("missing") || status.contains("not found"));
    }

    #[test]
    fn should_return_health_error_missing_cache_for_yazi() {
        let (_, mut ctx) = create_test_context();
        let generator = YaziGenerator;
        let theme: Theme = Theme::mock();
        ctx.state.current_theme = theme.name.clone();

        let mut task = ctx.log.step("Test", false).as_quiet();
        generator
            .apply(&theme, &ctx.paths, &ctx.templater, &mut task)
            .unwrap();
        let cache_path = generator.cache_path(&ctx.paths, &theme.name);
        fs::remove_file(cache_path).unwrap();

        let status = generator.health_check(&ctx.paths, &theme.name);
        assert!(status.is_error(), "Expected Error, got: {status}");
        assert!(status.contains("cache") && status.contains("missing"));
    }

    #[test]
    fn should_apply_theme_for_yazi() {
        let (_, ctx) = create_test_context();
        let generator = YaziGenerator;
        let theme: Theme = Theme::mock();

        let mut task = ctx.log.step("Test", false);
        generator
            .apply(&theme, &ctx.paths, &ctx.templater, &mut task)
            .unwrap();

        let expected_yazi_dir = generator.resolve_config_directory(&ctx.paths);
        let yazi_theme_link = expected_yazi_dir.join("theme.toml");

        assert!(
            yazi_theme_link.exists(),
            "Symlink missing at {:?}. Check if resolve_config_directory is consistent!",
            yazi_theme_link
        );

        let cache_content = fs::read_to_string(yazi_theme_link).unwrap();
        assert!(cache_content.contains("generated by Iris"));
    }

    #[test]
    fn should_fix_broken_symlink_for_yazi() {
        let (_, ctx) = create_test_context();
        let generator = YaziGenerator;
        let theme: Theme = Theme::mock();

        let mut task = ctx.log.step("Test", false);
        generator
            .apply(&theme, &ctx.paths, &ctx.templater, &mut task)
            .unwrap();

        let link_path = generator.link_path(&ctx.paths, &theme.name);
        let cache_file = generator.cache_path(&ctx.paths, &theme.name);

        fs::remove_file(&link_path).unwrap();

        let status = generator.health_check(&ctx.paths, &theme.name);
        assert!(status.is_error());
        assert!(status.contains("link missing"));

        generator
            .fix(&status, &theme, &ctx.paths, &ctx.templater, &mut task)
            .unwrap();
        assert!(link_path.exists(), "Fix should recreate the symlink");

        #[cfg(unix)]
        {
            let target = fs::read_link(&link_path).unwrap();
            assert_eq!(
                target, cache_file,
                "Symlink should point back to the cache file"
            );
        }

        assert!(generator.health_check(&ctx.paths, &theme.name).is_ok());
    }

    #[test]
    fn should_fix_missing_cache_for_yazi() {
        let (_, ctx) = create_test_context();
        let generator = YaziGenerator;
        let theme: Theme = Theme::mock();
        let mut task = ctx.log.step("Test", false).as_quiet();

        generator
            .apply(&theme, &ctx.paths, &ctx.templater, &mut task)
            .unwrap();

        let cache_file = generator.cache_path(&ctx.paths, &theme.name);
        fs::remove_file(&cache_file).unwrap();

        let status = generator.health_check(&ctx.paths, &theme.name);
        assert!(status.is_error());
        assert!(status.contains("cache") && status.contains("missing"));

        generator
            .fix(&status, &theme, &ctx.paths, &ctx.templater, &mut task)
            .unwrap();
        assert!(cache_file.exists());
        assert!(generator.health_check(&ctx.paths, &theme.name).is_ok());
    }

    #[test]
    fn should_clear_generated_files_for_yazi() {
        let (_, ctx) = create_test_context();
        let generator = YaziGenerator;
        let theme: Theme = Theme::mock();

        let mut task = ctx.log.step("Test", false);
        generator
            .apply(&theme, &ctx.paths, &ctx.templater, &mut task)
            .unwrap();

        let cache_dir = ctx.paths.generators.join(generator.name());
        assert!(cache_dir.exists());

        generator.clear(&ctx.paths).unwrap();
        assert!(
            !cache_dir.exists(),
            "Clear should remove the entire cache dir"
        );
    }

    #[test]
    fn should_remove_theme_for_yazi() {
        let (_, ctx) = create_test_context();
        let generator = YaziGenerator;
        let theme: Theme = Theme::mock();

        let mut task = ctx.log.step("Test", false);
        generator
            .apply(&theme, &ctx.paths, &ctx.templater, &mut task)
            .unwrap();

        let cache_file = generator.cache_path(&ctx.paths, &theme.name);
        let link_file = generator.theme_path(&ctx.paths, &theme.name);

        assert!(cache_file.exists());
        assert!(link_file.exists());

        generator.remove_theme(&ctx.paths, &theme.name).unwrap();

        assert!(!cache_file.exists());
        assert!(!link_file.exists());
    }
}
