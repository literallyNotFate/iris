use crate::{
    core::{IrisPaths, Templater},
    log::Task,
    models::{HealthStatus, Theme},
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
        theme: &Theme,
        paths: &IrisPaths,
        templater: &Templater,
        task: &mut Task,
    ) -> Result<()> {
        task.info(&format!(
            "Generating {} theme for {}",
            theme.name.yellow(),
            self.name().bold().cyan(),
        ));

        let cache_file: PathBuf = self.ensure_cache_file(theme, paths, templater)?;
        let link_path: PathBuf = self.link_path(paths, &theme.name);

        task.info(&format!(
            "Linking {} theme to {}...",
            self.name().bold(),
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
        c.insert("bg", &theme.palette.bg);
        c.insert("fg", &theme.palette.fg);
        c.insert("white", &theme.palette.white);
        c.insert("sel", &theme.palette.sel);
        c.insert("ansi", &theme.palette.ansi);
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
        let symlink_status = HealthStatus::check_symlink(&link_path, "Theme link");

        if symlink_status.is_error() {
            return HealthStatus::error(
                "Theme link missing in `alacritty` config directory",
                Some("run `iris sync` or `iris health --fix` to regenerate"),
            );
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
                    return HealthStatus::error(
                        "Theme is not imported in alacritty.toml",
                        Some("Add `import = [\"~/.config/alacritty/current_theme.toml\"]`"),
                    );
                }
            }
            Err(e) => {
                return HealthStatus::Warning(format!("Could not read alacritty.toml: {e}"));
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
        if status.contains("link missing") {
            task.log.action("Restored `alacritty` theme symlink", || {
                let cache = self.cache_path(paths, "");
                let link = self.link_path(paths, "");
                self.ensure_symlink(&cache, &link)
            })?;
            fixed = true;
        }

        if status.contains("import") {
            task.log
                .action("Injected theme import into alacritty.toml", || {
                    self.inject_import_line(paths)
                })?;
            fixed = true;
        }

        if status.contains("unexpected") || status.contains("old") {
            fixed = false;
        }

        if !fixed {
            if status.is_warning() && !status.contains("unexpected") && !status.contains("old") {
                task.info(&format!("Fixing `alacritty` warning: {status}"));
            }

            task.log
                .action("Regenerating complete `alacritty` configuration", || {
                    self.apply(theme, paths, templater, &mut task.as_quiet())
                })?;
        }

        Ok(())
    }
}

impl AlacrittyGenerator {
    fn ensure_cache_file(
        &self,
        theme: &Theme,
        paths: &IrisPaths,
        templater: &Templater,
    ) -> Result<PathBuf> {
        let cache_file: PathBuf = self.cache_path(paths, &theme.name);
        let render_ctx = self.build_render_context(theme);
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
        let theme: Theme = Theme::mock();
        let ctx = generator.build_render_context(&theme);

        assert_eq!(
            ctx.get("bg")
                .expect("bg not found in context")
                .as_str()
                .unwrap(),
            theme.palette.bg
        );
        assert_eq!(
            ctx.get("fg")
                .expect("fg not found in context")
                .as_str()
                .unwrap(),
            theme.palette.fg
        );
        assert!(ctx.contains_key("ansi"));
    }

    #[test]
    fn should_return_health_ok_for_alacritty() {
        let (_, mut ctx) = create_test_context();
        let generator = AlacrittyGenerator;
        let theme: Theme = Theme::mock();

        let mut task = ctx.log.step("Test", false).as_quiet();
        ctx.state.current_theme = theme.name.clone();
        generator
            .apply(&theme, &ctx.paths, &ctx.templater, &mut task)
            .unwrap();

        let alacritty_dir = generator.resolve_config_directory(&ctx.paths);
        let main_config = alacritty_dir.join("alacritty.toml");
        fs::write(&main_config, "import = [\"current_theme.toml\"]").unwrap();

        let status = generator.health_check(&ctx.paths, &theme.name);
        assert!(status.is_ok(), "Expected Ok, got: {status}");
    }

    #[test]
    fn should_return_health_error_no_import_for_alacritty() {
        let (_, mut ctx) = create_test_context();
        let generator = AlacrittyGenerator;
        let theme: Theme = Theme::mock();

        let mut task = ctx.log.step("Test", false).as_quiet();
        ctx.state.current_theme = theme.name.clone();
        generator
            .apply(&theme, &ctx.paths, &ctx.templater, &mut task)
            .unwrap();

        let main_config = generator
            .resolve_config_directory(&ctx.paths)
            .join("alacritty.toml");
        fs::write(&main_config, "[window]\ndecorations = \"none\"").unwrap();

        let status = generator.health_check(&ctx.paths, &theme.name);

        assert!(
            status.is_error(),
            "Expected Error for missing import, got: {status}"
        );
        assert!(status.contains("not imported"));
    }

    #[test]
    fn should_return_health_warning_no_main_config_for_alacritty() {
        let (_, mut ctx) = create_test_context();
        let generator = AlacrittyGenerator;
        let theme: Theme = Theme::mock();

        let mut task = ctx.log.step("Test", false).as_quiet();
        ctx.state.current_theme = theme.name.clone();
        generator
            .apply(&theme, &ctx.paths, &ctx.templater, &mut task)
            .unwrap();

        let main_config = generator
            .resolve_config_directory(&ctx.paths)
            .join("alacritty.toml");
        if main_config.exists() {
            fs::remove_file(main_config).unwrap();
        }

        let status = generator.health_check(&ctx.paths, &theme.name);

        assert!(
            status.is_warning(),
            "Expected Warning for missing file, got: {status}"
        );
        assert!(status.contains("not found"));
    }

    #[test]
    fn should_apply_theme_for_alacritty() {
        let (_, ctx) = create_test_context();
        let generator = AlacrittyGenerator;
        let theme: Theme = Theme::mock();

        let mut task = ctx.log.step("Test", false).as_quiet();
        let result = generator.apply(&theme, &ctx.paths, &ctx.templater, &mut task);
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

        let content = fs::read_to_string(cache_file).unwrap();
        assert!(content.contains(&format!("background = \"{}\"", theme.palette.bg)));
        assert!(content.contains(&format!("black   = \"{}\"", theme.palette.ansi[0])));
        assert!(content.contains(&format!("white   = \"{}\"", theme.palette.ansi[15])));
    }

    #[test]
    fn should_fix_inject_issue_for_alacritty() {
        let (_, ctx) = create_test_context();
        let generator = AlacrittyGenerator;
        let theme: Theme = Theme::mock();

        let alacritty_dir = generator.resolve_config_directory(&ctx.paths);
        fs::create_dir_all(&alacritty_dir).unwrap();
        let config_path = alacritty_dir.join("alacritty.toml");

        let mut task = ctx.log.step("Test", false).as_quiet();
        generator
            .apply(&theme, &ctx.paths, &ctx.templater, &mut task)
            .unwrap();
        fs::write(&config_path, "[window]\ndecorations = \"none\"").unwrap();

        let status = generator.health_check(&ctx.paths, &theme.name);

        assert!(
            status.is_error(),
            "Expected Import Error, but got: {status}"
        );
        assert!(status.contains("import"));

        generator
            .fix(&status, &theme, &ctx.paths, &ctx.templater, &mut task)
            .expect("Fix failed");

        let content = fs::read_to_string(&config_path).unwrap();
        assert!(
            content.contains("current_theme.toml"),
            "Import line missing after fix!"
        );
        assert!(generator.health_check(&ctx.paths, &theme.name).is_ok());
    }

    #[test]
    fn should_fix_broken_symlink_for_alacritty() {
        let (_, ctx) = create_test_context();
        let generator = AlacrittyGenerator;
        let theme: Theme = Theme::mock();

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
            .apply(&theme, &ctx.paths, &ctx.templater, &mut task)
            .unwrap();

        let link_path = generator.link_path(&ctx.paths, "");
        if link_path.exists() || link_path.is_symlink() {
            fs::remove_file(&link_path).unwrap();
        }

        let status = generator.health_check(&ctx.paths, &theme.name);
        assert!(status.is_error(), "Should be Error, got: {status}");

        generator
            .fix(&status, &theme, &ctx.paths, &ctx.templater, &mut task)
            .expect("Fix failed");

        let final_status = generator.health_check(&ctx.paths, &theme.name);
        assert!(
            final_status.is_ok(),
            "Health check failed after fix: {final_status}"
        );
    }

    #[test]
    fn should_clear_generated_files_for_alacritty() {
        let (_, ctx) = create_test_context();
        let generator = AlacrittyGenerator;

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
    fn should_remove_theme_for_alacritty() {
        let (_, ctx) = create_test_context();
        let generator = AlacrittyGenerator;

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
