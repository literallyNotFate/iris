use crate::{
    core::{IrisPaths, Templater},
    models::{HealthStatus, Palette},
    modules::{Generator, GeneratorType},
    ui::Logger,
    utils::{self},
};
use anyhow::{Context, Result};
use colored::Colorize;
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Config generator for ghostty terminal
pub struct GhosttyGenerator;

impl Generator for GhosttyGenerator {
    fn name(&self) -> &str {
        "ghostty"
    }

    fn generator_type(&self) -> GeneratorType {
        GeneratorType::Terminal
    }

    fn target_file_name(&self, _theme: &str) -> String {
        "current_theme.conf".into()
    }

    fn cache_path(&self, paths: &IrisPaths, _theme: &str) -> PathBuf {
        paths
            .generators
            .join(self.name())
            .join(self.target_file_name(""))
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
        log: &Logger,
    ) -> Result<()> {
        log.info(&format!(
            "Generating {} theme for {}",
            utils::capitalize(&p.name).yellow(),
            self.name().bold().cyan(),
        ));
        let cache_file: PathBuf = self.ensure_cache_file(p, paths, templater)?;
        let link_path: PathBuf = self.link_path(paths, &p.name);

        log.info(&format!(
            "Linking {} theme to {}...",
            self.name().bold(),
            utils::pretty_path(&link_path).magenta(),
        ));

        self.ensure_symlink(&cache_file, &link_path)?;

        log.info(&format!(
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
        c.insert("sel_bg", &p.sel);
        c.insert("sel_fg", &p.bg);
        c.insert("cursor", &p.white);
        c.insert("ansi", &p.ansi);
        c
    }

    fn health_check(&self, paths: &IrisPaths, _theme: &str) -> HealthStatus {
        if !self.is_installed() {
            return HealthStatus::Warning("`ghostty` binary not found".into());
        }

        let ghostty_dir: PathBuf = self.resolve_config_directory(paths);
        let config_path: PathBuf = ghostty_dir.join("config");
        let link_path: PathBuf = self.link_path(paths, "");
        let expected_cache: PathBuf = self.cache_path(paths, "");

        if !link_path.exists() {
            return HealthStatus::Error {
                message: "current_theme.conf missing".into(),
                fix_hint: Some("run `iris sync` or `iris health --fix` to create the link".into()),
            };
        }

        if !config_path.exists() {
            return HealthStatus::Error {
                message: "`ghostty` config file missing".into(),
                fix_hint: Some(format!("Create {}", config_path.display())),
            };
        }

        let content = fs::read_to_string(&config_path).unwrap_or_default();
        let import_line = format!("config-file = {}", self.target_file_name(""));

        if !content.contains(&import_line) {
            return HealthStatus::Warning(format!(
                "Theme is generated but not imported in {}",
                config_path.display()
            ));
        }

        #[cfg(unix)]
        if let Ok(target) = fs::read_link(&link_path) {
            if target != expected_cache {
                return HealthStatus::Warning("Link points to a different cache location".into());
            }
        }

        HealthStatus::Ok
    }

    fn fix(
        &self,
        status: &HealthStatus,
        p: &Palette,
        paths: &IrisPaths,
        templater: &Templater,
        log: &Logger,
    ) -> Result<()> {
        match status {
            HealthStatus::Error { message, .. } => {
                let msg_low: String = message.to_lowercase();
                if msg_low.contains("missing") || msg_low.contains("not found") {
                    log.step("Repairing `ghostty` configuration and paths...", 2)
                        .done(true);

                    let cache = self.cache_path(paths, &p.name);
                    let link = self.link_path(paths, &p.name);
                    self.ensure_symlink(&cache, &link)?;
                }

                self.apply(p, paths, templater, &Logger::quiet())
            }
            HealthStatus::Warning(msg) if msg.contains("not imported") => {
                log.step("Injecting import into `ghostty`...", 2).done(true);
                self.inject_import_line(paths)
            }
            _ => {
                log.step(
                    &format!("Re-applying `{}` configuration...", self.name().bold()),
                    2,
                )
                .done(true);
                self.apply(p, paths, templater, &Logger::quiet())
            }
        }
    }
}

impl GhosttyGenerator {
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
                format!("Failed to create `ghostty` directory: {}", parent.display())
            })?;
        }

        fs::write(&cache_file, content).with_context(|| {
            format!(
                "Failed to write `ghostty` cache file: {}",
                cache_file.display()
            )
        })?;
        Ok(cache_file)
    }

    fn ensure_symlink(&self, target: &Path, link: &Path) -> anyhow::Result<()> {
        if link.exists() || link.is_symlink() {
            fs::remove_file(link).with_context(|| {
                format!(
                    "Failed to remove existing `ghostty` file/link at {}",
                    link.display()
                )
            })?;
        }

        if let Some(parent) = link.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "Failed to create parent directory for `ghostty` link: {}",
                    parent.display()
                )
            })?;
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            symlink(target, link).with_context(|| {
                format!(
                    "Failed to create `ghostty` symlink: {} -> {}",
                    target.display(),
                    link.display()
                )
            })?;
        }
        Ok(())
    }

    fn inject_import_line(&self, paths: &IrisPaths) -> anyhow::Result<()> {
        let config_path: PathBuf = self.resolve_config_directory(paths).join("config");
        let import_line: String = format!("config-file = {}", self.target_file_name(""));

        if !config_path.exists() {
            if let Some(parent) = config_path.parent() {
                fs::create_dir_all(parent).with_context(|| {
                    format!(
                        "Failed to create parent `ghostty` directory: {}",
                        parent.display()
                    )
                })?;
            }

            fs::write(&config_path, format!("{}\n", import_line)).with_context(|| {
                format!(
                    "Failed to create `ghostty` config file: {}",
                    config_path.display()
                )
            })?;
            return Ok(());
        }

        let content = fs::read_to_string(&config_path).with_context(|| {
            format!("Failed to read `ghostty` config: {}", config_path.display())
        })?;

        if !content.contains(&import_line) {
            use std::fs::OpenOptions;
            use std::io::Write;

            let mut file = OpenOptions::new()
                .append(true)
                .open(&config_path)
                .with_context(|| {
                    format!(
                        "Failed to open `ghostty` config for appending: {}",
                        config_path.display()
                    )
                })?;

            writeln!(file, "\n{}", import_line).with_context(|| {
                format!(
                    "Failed to write to `ghostty` config: {}",
                    config_path.display()
                )
            })?;
        }

        Ok(())
    }
}

/// Unit-tests for ghostty
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::tests::create_test_context;

    #[test]
    fn should_return_ghostty_metadata() {
        let generator = GhosttyGenerator;
        assert_eq!(generator.name(), "ghostty");
        assert_eq!(generator.generator_type(), GeneratorType::Terminal);
        assert_eq!(generator.target_file_name("any"), "current_theme.conf");
    }

    #[test]
    fn should_build_valid_render_context() {
        let generator = GhosttyGenerator;
        let p = Palette::mock();
        let ctx = generator.build_render_context(&p);

        assert_eq!(ctx.get("bg").unwrap().as_str().unwrap(), p.bg);
        assert!(ctx.get("ansi").unwrap().is_array());
    }

    #[test]
    fn should_return_health_ok_for_ghostty() {
        let (_, mut ctx) = create_test_context();
        let generator = GhosttyGenerator;
        let p = Palette::mock();

        ctx.state.current_theme = p.name.clone();
        generator
            .apply(&p, &ctx.paths, &ctx.templater, &ctx.log)
            .unwrap();

        let config_path = generator
            .resolve_config_directory(&ctx.paths)
            .join("config");
        let import_line = format!("config-file = {}", generator.target_file_name(&p.name));
        fs::write(&config_path, import_line).unwrap();

        let status = generator.health_check(&ctx.paths, &p.name);
        assert!(
            matches!(status, HealthStatus::Ok),
            "Expected Ok, got {:?}",
            status
        );
    }

    #[test]
    fn should_return_health_error_missing_config_for_ghostty() {
        let (tmp_dir, ctx) = create_test_context();
        let generator = GhosttyGenerator;
        let root = tmp_dir.path();

        let config_dir = generator.resolve_config_directory(&ctx.paths);
        let theme_link = generator.link_path(&ctx.paths, "");

        assert!(
            config_dir.starts_with(root),
            "Config dir {:?} is outside of root {:?}",
            config_dir,
            root
        );

        fs::create_dir_all(theme_link.parent().unwrap()).unwrap();
        fs::create_dir_all(&config_dir).unwrap();
        fs::write(&theme_link, "palette = []").unwrap();

        let config_path = config_dir.join("config");
        if config_path.exists() {
            fs::remove_file(&config_path).unwrap();
        }

        let status = generator.health_check(&ctx.paths, &ctx.state.current_theme);
        match status {
            HealthStatus::Error { ref message, .. } => {
                assert!(
                    message.to_lowercase().contains("config"),
                    "Message was: {}",
                    message
                );
            }
            _ => panic!("Expected Error for missing config, but got: {:?}", status),
        }
    }

    #[test]
    fn should_return_health_warning_no_import_for_ghostty() {
        let (_, mut ctx) = create_test_context();
        let generator = GhosttyGenerator;
        let p = Palette::mock();

        ctx.state.current_theme = p.name.clone();
        generator
            .apply(&p, &ctx.paths, &ctx.templater, &ctx.log)
            .unwrap();

        let config_path = generator
            .resolve_config_directory(&ctx.paths)
            .join("config");
        fs::write(&config_path, "font-family = JetBrainsMono").unwrap();

        let status = generator.health_check(&ctx.paths, &p.name);
        match status {
            HealthStatus::Warning(msg) => {
                assert!(msg.contains("not imported"));
            }
            _ => panic!("Expected Warning for missing import line, got {:?}", status),
        }
    }

    #[test]
    fn should_apply_theme_for_ghostty() {
        let (_, ctx) = create_test_context();
        let generator = GhosttyGenerator;
        let p = Palette::mock();

        let result = generator.apply(&p, &ctx.paths, &ctx.templater, &ctx.log);
        assert!(result.is_ok(), "Failed to apply: {:?}", result.err());

        let cache_file = ctx
            .paths
            .generators
            .join("ghostty")
            .join("current_theme.conf");
        assert!(cache_file.exists());

        let content = fs::read_to_string(cache_file).unwrap();
        assert!(content.contains("background ="));
        assert!(content.contains("palette = 0="));
        assert!(content.contains("palette = 15="));
    }

    #[test]
    fn should_fix_inject_issue_for_ghostty() {
        let (_, ctx) = create_test_context();
        let generator = GhosttyGenerator;
        let p = Palette::mock();

        let config_dir = generator.resolve_config_directory(&ctx.paths);
        fs::create_dir_all(&config_dir).unwrap();
        let config_path = config_dir.join("config");

        generator
            .apply(&p, &ctx.paths, &ctx.templater, &ctx.log)
            .unwrap();
        fs::write(&config_path, "font-size = 12").unwrap();

        let status = generator.health_check(&ctx.paths, &p.name);
        generator
            .fix(&status, &p, &ctx.paths, &ctx.templater, &ctx.log)
            .expect("Fix failed");

        let content = fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("current_theme.conf"));
        assert!(generator.health_check(&ctx.paths, &p.name).is_ok());
    }

    #[test]
    fn should_fix_broken_link_for_ghostty() {
        let (_, ctx) = create_test_context();
        let generator = GhosttyGenerator;
        let p = Palette::mock();
        let config_dir = generator.resolve_config_directory(&ctx.paths);
        fs::create_dir_all(&config_dir).unwrap();

        generator
            .apply(&p, &ctx.paths, &ctx.templater, &ctx.log)
            .unwrap();

        let config_path = config_dir.join("config");
        fs::write(&config_path, "config-import = current_theme.conf").unwrap();

        let link_path = generator.link_path(&ctx.paths, "");
        if link_path.exists() || link_path.is_symlink() {
            fs::remove_file(&link_path).unwrap();
        }

        let status = generator.health_check(&ctx.paths, &p.name);
        assert!(
            matches!(status, HealthStatus::Error { .. }),
            "Expected Error, got {:?}",
            status
        );

        generator
            .fix(&status, &p, &ctx.paths, &ctx.templater, &ctx.log)
            .expect("Fix failed");

        let final_status = generator.health_check(&ctx.paths, &p.name);
        assert!(link_path.exists(), "Link should exist");

        match final_status {
            HealthStatus::Error { message, .. } => {
                panic!("Fix failed, still have Error: {}", message)
            }
            _ => {}
        }
    }
}
