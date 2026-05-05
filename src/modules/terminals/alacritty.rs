use crate::{
    core::{IrisPaths, Templater},
    log::Task,
    models::{HealthStatus, Palette},
    modules::{Generator, GeneratorType},
    utils::{self},
};
use anyhow::{Context, Result};
use colored::Colorize;
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Config generator for Alacritty terminal
pub struct AlacrittyGenerator;

impl Generator for AlacrittyGenerator {
    fn name(&self) -> &str {
        "alacritty"
    }

    fn generator_type(&self) -> GeneratorType {
        GeneratorType::Terminal
    }

    fn target_file_name(&self, _theme: &str) -> String {
        "current_theme.toml".into()
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
        task: &mut Task,
    ) -> Result<()> {
        task.info(&format!(
            "Generating {} theme for {}",
            utils::capitalize(&p.name).yellow(),
            self.name().bold().cyan(),
        ));

        let cache_file: PathBuf = self.ensure_cache_file(p, paths, templater)?;
        let link_path: PathBuf = self.link_path(paths, &p.name);

        task.info(&format!(
            "Linking {} theme to {}...",
            self.name().bold(),
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
        c.insert("sel", &p.sel);
        c.insert("ansi", &p.ansi);
        c
    }

    fn health_check(&self, paths: &IrisPaths, _theme: &str) -> HealthStatus {
        if !self.is_installed() {
            return HealthStatus::Warning("`alacritty` binary not found".into());
        }

        let alacritty_dir: PathBuf = self.resolve_config_directory(paths);
        let main_config: PathBuf = alacritty_dir.join("alacritty.toml");
        let link_path: PathBuf = self.link_path(paths, "");
        let expected_cache: PathBuf = self.cache_path(paths, "");

        if !link_path.exists() && !link_path.is_symlink() {
            return HealthStatus::Error {
                message: "Theme link missing in `alacritty` config directory".into(),
                fix_hint: Some("run `iris sync` or `iris health --fix` to regenerate".into()),
            };
        }

        #[cfg(unix)]
        if let Ok(target) = fs::read_link(&link_path) {
            if target != expected_cache {
                return HealthStatus::Warning("Link points to an unexpected location".into());
            }
        }

        if !main_config.exists() {
            return HealthStatus::Warning(
                "alacritty.toml not found (using default settings)".into(),
            );
        }

        match fs::read_to_string(&main_config) {
            Ok(content) => {
                if !content.contains("current_theme.toml") {
                    return HealthStatus::Error {
                        message: "Theme is not imported in alacritty.toml".into(),
                        fix_hint: Some(
                            "Add `import = [\"~/.config/alacritty/current_theme.toml\"]`".into(),
                        ),
                    };
                }
            }
            Err(e) => {
                return HealthStatus::Warning(format!("Could not read alacritty.toml: {}", e));
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
        task: &mut Task,
    ) -> Result<()> {
        match status {
            HealthStatus::Error { message, .. } => {
                let message_low = message.to_lowercase();

                if message_low.contains("link missing") {
                    task.log.action("Restored `alacritty` theme symlink", || {
                        let cache = self.cache_path(paths, "");
                        let link = self.link_path(paths, "");
                        self.ensure_symlink(&cache, &link)
                    })?;
                }

                if message_low.contains("import") {
                    task.log
                        .action("Injected theme import into alacritty.toml", || {
                            self.inject_import_line(paths)
                        })?;
                }

                self.apply(p, paths, templater, &mut task.as_quiet())
            }

            HealthStatus::Warning(msg) => {
                let msg_low = msg.to_lowercase();

                if msg_low.contains("unexpected") || msg_low.contains("old") {
                    task.log.action("Updated `alacritty` symlink target", || {
                        self.apply(p, paths, templater, &mut task.as_quiet())
                    })
                } else {
                    task.info(&format!("Fixing `alacritty` warning: {}", msg));
                    self.apply(p, paths, templater, &mut task.as_quiet())
                }
            }

            _ => task.log.action(
                &format!("Re-applied `{}` configuration", self.name().bold()),
                || self.apply(p, paths, templater, &mut task.as_quiet()),
            ),
        }
    }
}

impl AlacrittyGenerator {
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
                    "Failed to create `alacritty` directory: {}",
                    parent.display()
                )
            })?;
        }

        fs::write(&cache_file, content).with_context(|| {
            format!(
                "Failed to write `alacritty` theme: {}",
                cache_file.display()
            )
        })?;
        Ok(cache_file)
    }

    fn ensure_symlink(&self, target: &Path, link: &Path) -> Result<()> {
        if link.exists() || link.is_symlink() {
            fs::remove_file(link).with_context(|| {
                format!(
                    "Failed to remove old `alacritty` theme link: {}",
                    link.display()
                )
            })?;
        }

        if let Some(parent) = link.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "Failed to create parent directory for `alacritty` link: {}",
                    parent.display()
                )
            })?;
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            symlink(target, link).with_context(|| {
                format!(
                    "Failed to create `alacritty` symlink: {} -> {}",
                    target.display(),
                    link.display()
                )
            })?;
        }
        Ok(())
    }

    fn inject_import_line(&self, paths: &IrisPaths) -> Result<()> {
        let config_path: PathBuf = self.resolve_config_directory(paths).join("alacritty.toml");
        let import_line: &str = "import = [\"~/.config/alacritty/current_theme.toml\"]";

        if !config_path.exists() {
            if let Some(parent) = config_path.parent() {
                fs::create_dir_all(parent).with_context(|| {
                    format!(
                        "Failed to create `alacritty` config directory: {}",
                        parent.display()
                    )
                })?;
            }

            fs::write(
                &config_path,
                format!("# Alacritty Config\n{}\n", import_line),
            )
            .with_context(|| {
                format!(
                    "Failed to create `alacritty` config: {}",
                    config_path.display()
                )
            })?;
        } else {
            let content: String = fs::read_to_string(&config_path).with_context(|| {
                format!(
                    "Failed to read `alacritty` config: {}",
                    config_path.display()
                )
            })?;

            if !content.contains("current_theme.toml") {
                let new_content: String = format!("{}\n\n{}", import_line, content);
                fs::write(&config_path, new_content).with_context(|| {
                    format!(
                        "Failed to update `alacritty` config: {}",
                        config_path.display()
                    )
                })?;
            }
        }

        Ok(())
    }
}

/// Unit-tests for alacritty
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::tests::create_test_context;

    #[test]
    fn should_return_alacritty_metadata() {
        let generator = AlacrittyGenerator;
        assert_eq!(generator.name(), "alacritty");
        assert_eq!(generator.generator_type(), GeneratorType::Terminal);
        assert_eq!(generator.target_file_name("nord"), "current_theme.toml");
    }

    #[test]
    fn should_build_valid_render_context() {
        let generator = AlacrittyGenerator;
        let p = Palette::mock();
        let ctx = generator.build_render_context(&p);

        assert_eq!(
            ctx.get("bg")
                .expect("bg not found in context")
                .as_str()
                .unwrap(),
            p.bg
        );
        assert_eq!(
            ctx.get("fg")
                .expect("fg not found in context")
                .as_str()
                .unwrap(),
            p.fg
        );
        assert!(ctx.contains_key("ansi"));
    }

    #[test]
    fn should_return_health_ok_for_alacritty() {
        let (_, mut ctx) = create_test_context();
        let generator = AlacrittyGenerator;
        let p = Palette::mock();

        let mut task = ctx.log.step("Test", false).as_quiet();
        ctx.state.current_theme = p.name.clone();
        generator
            .apply(&p, &ctx.paths, &ctx.templater, &mut task)
            .unwrap();

        let alacritty_dir = generator.resolve_config_directory(&ctx.paths);
        let main_config = alacritty_dir.join("alacritty.toml");
        fs::write(&main_config, "import = [\"current_theme.toml\"]").unwrap();

        let status = generator.health_check(&ctx.paths, &p.name);
        assert!(
            matches!(status, HealthStatus::Ok),
            "Expected Ok, got {:?}",
            status
        );
    }

    #[test]
    fn should_return_health_error_no_import_for_alacritty() {
        let (_, mut ctx) = create_test_context();
        let generator = AlacrittyGenerator;
        let p = Palette::mock();

        let mut task = ctx.log.step("Test", false).as_quiet();
        ctx.state.current_theme = p.name.clone();
        generator
            .apply(&p, &ctx.paths, &ctx.templater, &mut task)
            .unwrap();

        let main_config = generator
            .resolve_config_directory(&ctx.paths)
            .join("alacritty.toml");
        fs::write(&main_config, "[window]\ndecorations = \"none\"").unwrap();

        let status = generator.health_check(&ctx.paths, &p.name);
        match status {
            HealthStatus::Error { ref message, .. } => {
                assert!(message.contains("not imported"));
            }
            _ => panic!("Expected Error for missing import, got {:?}", status),
        }
    }

    #[test]
    fn should_return_health_warning_no_main_config_for_alacritty() {
        let (_, mut ctx) = create_test_context();
        let generator = AlacrittyGenerator;
        let p = Palette::mock();

        let mut task = ctx.log.step("Test", false).as_quiet();
        ctx.state.current_theme = p.name.clone();
        generator
            .apply(&p, &ctx.paths, &ctx.templater, &mut task)
            .unwrap();

        let main_config = generator
            .resolve_config_directory(&ctx.paths)
            .join("alacritty.toml");
        if main_config.exists() {
            fs::remove_file(main_config).unwrap();
        }

        let status = generator.health_check(&ctx.paths, &p.name);
        assert!(matches!(status, HealthStatus::Warning(msg) if msg.contains("not found")));
    }

    #[test]
    fn should_apply_theme_for_alacritty() {
        let (_, ctx) = create_test_context();
        let generator = AlacrittyGenerator;
        let p = Palette::mock();

        let mut task = ctx.log.step("Test", false).as_quiet();
        let result = generator.apply(&p, &ctx.paths, &ctx.templater, &mut task);
        assert!(result.is_ok(), "Apply failed: {:?}", result.err());

        let cache_file = ctx
            .paths
            .generators
            .join("alacritty")
            .join("current_theme.toml");
        assert!(cache_file.exists(), "Theme missing in Iris cache");

        let alacritty_dir = generator.resolve_config_directory(&ctx.paths);
        let link_path = alacritty_dir.join("current_theme.toml");
        assert!(
            link_path.exists(),
            "Symlink missing in Alacritty config dir"
        );
        assert!(cache_file.exists());

        let content = fs::read_to_string(cache_file).unwrap();
        assert!(content.contains(&format!("background = \"{}\"", p.bg)));
        assert!(content.contains(&format!("black   = \"{}\"", p.ansi[0])));
        assert!(content.contains(&format!("white   = \"{}\"", p.ansi[15])));
    }

    #[test]
    fn should_fix_inject_issue_for_alacritty() {
        let (_, ctx) = create_test_context();
        let generator = AlacrittyGenerator;
        let p = Palette::mock();

        let alacritty_dir = generator.resolve_config_directory(&ctx.paths);
        fs::create_dir_all(&alacritty_dir).unwrap();
        let config_path = alacritty_dir.join("alacritty.toml");

        let mut task = ctx.log.step("Test", false).as_quiet();
        generator
            .apply(&p, &ctx.paths, &ctx.templater, &mut task)
            .unwrap();
        fs::write(&config_path, "[window]\ndecorations = \"none\"").unwrap();

        let status = generator.health_check(&ctx.paths, &p.name);

        match status {
            HealthStatus::Error { ref message, .. }
                if message.to_lowercase().contains("import") => {}
            _ => panic!("Expected Import Error, but got: {:?}", status),
        }

        generator
            .fix(&status, &p, &ctx.paths, &ctx.templater, &mut task)
            .expect("Fix failed");

        let content = fs::read_to_string(&config_path).unwrap();
        assert!(
            content.contains("current_theme.toml"),
            "Import line missing after fix!"
        );

        let final_status = generator.health_check(&ctx.paths, &p.name);
        assert!(
            final_status.is_ok(),
            "Final status should be Ok, but got: {:?}",
            final_status
        );
    }

    #[test]
    fn should_fix_broken_symlink_for_alacritty() {
        let (_, ctx) = create_test_context();
        let generator = AlacrittyGenerator;
        let p = Palette::mock();

        let alacritty_dir = generator.resolve_config_directory(&ctx.paths);
        fs::create_dir_all(&alacritty_dir).unwrap();

        let config_path = alacritty_dir.join("alacritty.toml");
        fs::write(
            &config_path,
            "import = [\"~/.config/alacritty/current_theme.toml\"]",
        )
        .unwrap();

        let mut task = ctx.log.step("Test", false).as_quiet();
        generator
            .apply(&p, &ctx.paths, &ctx.templater, &mut task)
            .unwrap();

        let link_path = generator.link_path(&ctx.paths, "");
        if link_path.exists() || link_path.is_symlink() {
            fs::remove_file(&link_path).unwrap();
        }

        let status = generator.health_check(&ctx.paths, &p.name);
        assert!(
            matches!(status, HealthStatus::Error { .. }),
            "Should be Error, got: {:?}",
            status
        );

        generator
            .fix(&status, &p, &ctx.paths, &ctx.templater, &mut task)
            .expect("Fix failed");

        let final_status = generator.health_check(&ctx.paths, &p.name);
        assert!(
            final_status.is_ok(),
            "Health check failed after fix: {:?}",
            final_status
        );
    }
}
