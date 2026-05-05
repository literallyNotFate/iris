use crate::{
    core::{IrisPaths, Templater},
    log::Task,
    models::{HealthStatus, Palette},
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
        p: &Palette,
        paths: &IrisPaths,
        templater: &Templater,
        task: &mut Task,
    ) -> Result<()> {
        let cache_file: PathBuf = self.ensure_cache_file(p, paths, templater)?;
        let link_path: PathBuf = self.link_path(paths, &p.name);

        task.info(&format!(
            "Linking {} theme to {}...",
            self.name().bold().cyan(),
            utils::pretty_path(&link_path).magenta(),
        ));
        self.ensure_symlink(&cache_file, &link_path)?;

        task.info(&format!(
            "{} theme applied to {}",
            utils::capitalize(&p.name).yellow(),
            self.name().bold().cyan()
        ));
        Ok(())
    }

    fn build_render_context(&self, p: &Palette) -> tera::Context {
        let mut c = tera::Context::new();

        c.insert("theme_name", &utils::capitalize(&p.name));
        c.insert("bg", &p.bg);
        c.insert("fg", &p.fg);
        c.insert("white", &p.white);
        c.insert("comment", &p.comment);
        c.insert("gutter_fg", &p.gutter_fg);
        c.insert("ansi", &p.ansi);
        c.insert("sel", &p.sel);

        let line_hl = if p.line_hl == "#cccccc" {
            &p.sel
        } else {
            &p.line_hl
        };
        c.insert("line_hl", line_hl);

        c.insert("red", &p.ansi[1]);
        c.insert("green", &p.ansi[2]);
        c.insert("orange", &p.ansi[3]);
        c.insert("blue", &p.ansi[4]);
        c.insert("magenta", &p.ansi[5]);
        c.insert("teal", &p.ansi[6]);
        c.insert("tan", &p.ansi[7]);
        c.insert("br_red", &p.ansi[9]);
        c.insert("br_green", &p.ansi[10]);
        c.insert("br_orange", &p.ansi[11]);
        c.insert("br_blue", &p.ansi[12]);
        c.insert("br_magenta", &p.ansi[13]);
        c.insert("br_teal", &p.ansi[14]);

        c
    }

    fn health_check(&self, paths: &IrisPaths, _theme: &str) -> HealthStatus {
        if !self.is_installed() {
            return HealthStatus::Warning("`yazi` binary not found".into());
        }

        let link_path: PathBuf = self.link_path(paths, "");
        let expected_cache: PathBuf = self.cache_path(paths, "");

        if !link_path.exists() && !link_path.is_symlink() {
            return HealthStatus::Error {
                message: "theme.toml link missing in yazi config".into(),
                fix_hint: Some(
                    "run `iris sync` or `iris health --fix` to create the symlink".into(),
                ),
            };
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

        if !expected_cache.exists() {
            return HealthStatus::Error {
                message: "`yazi` theme cache file is missing".into(),
                fix_hint: Some("run `iris sync` or `iris health --fix` to regenerate".into()),
            };
        }

        HealthStatus::Ok
    }

    fn fix(
        &self,
        status: &HealthStatus,
        p: &Palette,
        paths: &IrisPaths,
        templater: &Templater,
        task: &mut Task,
    ) -> Result<()> {
        match status {
            HealthStatus::Error { message, .. } => {
                if message.contains("cache file is missing") {
                    task.log.action("Generated missing cache file", || {
                        self.ensure_cache_file(p, paths, templater)
                    })?;
                } else if message.contains("link missing") {
                    task.log.action("Restored missing symlink", || {
                        let cache = self.cache_path(paths, &p.name);
                        let link = self.link_path(paths, &p.name);
                        self.ensure_symlink(&cache, &link)
                    })?;
                }
                Ok(())
            }

            HealthStatus::Warning(msg) if msg.contains("unexpected location") => {
                task.log.action("Relinked theme to correct location", || {
                    let cache = self.cache_path(paths, &p.name);
                    let link = self.link_path(paths, &p.name);
                    self.ensure_symlink(&cache, &link)
                })
            }

            _ => task.log.action(
                &format!("Re-applied `{}` configuration", self.name().bold()),
                || self.apply(p, paths, templater, &mut task.as_quiet()),
            ),
        }
    }
}

impl YaziGenerator {
    fn ensure_cache_file(
        &self,
        p: &Palette,
        paths: &IrisPaths,
        templater: &Templater,
    ) -> Result<PathBuf> {
        let cache_file: PathBuf = self.cache_path(paths, &p.name);
        let render_ctx = self.build_render_context(p);
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
        let mut p = Palette::mock();

        p.line_hl = "#123456".to_string();
        let ctx = generator.build_render_context(&p);
        assert_eq!(ctx.get("line_hl").unwrap().as_str().unwrap(), "#123456");

        p.line_hl = "#cccccc".to_string();
        p.sel = "#ff0000".to_string();
        let ctx = generator.build_render_context(&p);

        assert_eq!(ctx.get("line_hl").unwrap().as_str().unwrap(), "#ff0000");
        assert!(ctx.get("red").is_some());
        assert!(ctx.get("br_teal").is_some());
    }

    #[test]
    fn should_return_health_ok_for_yazi() {
        let (_tmp_dir, mut ctx) = create_test_context();
        let generator = YaziGenerator;
        let p = Palette::mock();

        ctx.state.current_theme = p.name.clone();
        let mut task = ctx.log.step("Test", false);
        generator
            .apply(&p, &ctx.paths, &ctx.templater, &mut task)
            .unwrap();

        let status = generator.health_check(&ctx.paths, &p.name);
        assert!(matches!(status, HealthStatus::Ok));
    }

    #[test]
    fn should_return_health_error_missing_link_for_yazi() {
        let (_, ctx) = create_test_context();
        let generator = YaziGenerator;
        let link = generator.link_path(&ctx.paths, "");

        if link.exists() || link.is_symlink() {
            if link.is_dir() && !link.is_symlink() {
                fs::remove_dir_all(&link).unwrap();
            } else {
                fs::remove_file(&link).unwrap();
            }
        }

        let status = generator.health_check(&ctx.paths, &ctx.state.current_theme);
        match status {
            HealthStatus::Error { message, .. } => {
                assert!(
                    message.to_lowercase().contains("missing")
                        || message.to_lowercase().contains("not found"),
                    "Expected missing link error, but got: {}",
                    message
                );
            }
            _ => panic!(
                "Expected HealthStatus::Error (missing link) at {:?}, but got: {:?}",
                link, status
            ),
        }
    }

    #[test]
    fn should_return_health_error_missing_cache_for_yazi() {
        let (_, mut ctx) = create_test_context();
        let generator = YaziGenerator;
        let p = Palette::mock();

        ctx.state.current_theme = p.name.clone();
        let mut task = ctx.log.step("Test", false);
        generator
            .apply(&p, &ctx.paths, &ctx.templater, &mut task)
            .unwrap();

        let cache_path = generator.cache_path(&ctx.paths, &p.name);
        fs::remove_file(cache_path).unwrap();

        let status = generator.health_check(&ctx.paths, &p.name);
        match status {
            HealthStatus::Error { message, .. } => {
                assert!(message.contains("cache file is missing"));
            }
            _ => panic!("Expected Error, got {:?}", status),
        }
    }

    #[test]
    fn should_apply_theme_for_yazi() {
        let (_, ctx) = create_test_context();
        let generator = YaziGenerator;
        let p = Palette::mock();

        let mut task = ctx.log.step("Test", false);
        let result = generator.apply(&p, &ctx.paths, &ctx.templater, &mut task);
        assert!(result.is_ok(), "Apply failed: {:?}", result.err());

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
        let p = Palette::mock();

        let mut task = ctx.log.step("Test", false);
        generator
            .apply(&p, &ctx.paths, &ctx.templater, &mut task)
            .expect("Initial apply failed");

        let link_path = generator.link_path(&ctx.paths, &p.name);
        let cache_file = generator.cache_path(&ctx.paths, &p.name);

        assert!(link_path.exists(), "Link should exist after apply");
        assert!(cache_file.exists(), "Cache should exist after apply");

        fs::remove_file(&link_path).expect("Failed to break link");
        assert!(!link_path.exists());

        let status = generator.health_check(&ctx.paths, &p.name);
        assert!(
            matches!(status, HealthStatus::Error { ref message, .. } if message.contains("link missing")),
            "Expected link missing error, got {:?}",
            status
        );

        let mut task = ctx.log.step("Test", false);
        generator
            .fix(&status, &p, &ctx.paths, &ctx.templater, &mut task)
            .expect("Fix failed");
        assert!(link_path.exists(), "Fix should recreate the symlink");

        #[cfg(unix)]
        {
            let target = fs::read_link(&link_path).expect("Should be a readable symlink");
            assert_eq!(
                target, cache_file,
                "Symlink should point back to the cache file"
            );
        }

        let final_status = generator.health_check(&ctx.paths, &p.name);
        assert!(final_status.is_ok(), "Final health status should be Ok");
    }

    #[test]
    fn should_fix_missing_cache_for_yazi() {
        let (_, ctx) = create_test_context();
        let generator = YaziGenerator;
        let p = Palette::mock();

        let mut task = ctx.log.step("Test", false);
        generator
            .apply(&p, &ctx.paths, &ctx.templater, &mut task)
            .ok();
        let cache_file = generator.cache_path(&ctx.paths, &p.name);
        fs::remove_file(&cache_file).unwrap();

        let status = generator.health_check(&ctx.paths, &p.name);
        assert!(
            matches!(status, HealthStatus::Error { ref message, .. } if message.contains("cache file is missing"))
        );

        generator
            .fix(&status, &p, &ctx.paths, &ctx.templater, &mut task)
            .expect("Fix failed");
        assert!(
            cache_file.exists(),
            "Fix should regenerate the missing cache file"
        );
    }
}
