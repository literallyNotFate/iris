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

/// Config generator for btop utility
pub struct BtopGenerator;

impl Generator for BtopGenerator {
    fn name(&self) -> &str {
        "btop"
    }

    fn generator_type(&self) -> GeneratorType {
        GeneratorType::System
    }

    fn target_file_name(&self, theme: &str) -> String {
        format!("{}.theme", theme)
    }

    fn resolve_config_directory(&self, paths: &IrisPaths) -> PathBuf {
        let config_base: PathBuf = paths
            .config
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| paths.config.clone());

        config_base.join(self.name()).join("themes")
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

        let conf_path: PathBuf = self
            .resolve_config_directory(paths)
            .parent()
            .unwrap_or(&self.resolve_config_directory(paths))
            .join("btop.conf");

        if conf_path.exists() {
            task.info(&format!(
                "Setting color_theme = \"{}\" in btop.conf",
                p.name.bold().red()
            ));

            self.update_btop_conf(&conf_path, &p.name)?;
        }

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
        c.insert("sel", &p.sel);
        c.insert("white", &p.white);
        c.insert("comment", &p.comment);
        c.insert("line_hl", &p.line_hl);
        c.insert("keyword", &p.keyword);
        c.insert("type_name", &p.type_name);
        c.insert("func", &p.func);
        c.insert("tag", &p.tag);
        c.insert("string", &p.string);
        c.insert("constant", &p.constant);
        c.insert("attribute", &p.attribute);

        c.insert("green", &p.ansi[2]);
        c.insert("yellow", &p.ansi[3]);
        c.insert("orange", &p.ansi[9]);

        c
    }

    fn health_check(&self, paths: &IrisPaths, theme: &str) -> HealthStatus {
        if !self.is_installed() {
            return HealthStatus::Warning("`btop` binary not found".into());
        }

        let themes_dir: PathBuf = self.resolve_config_directory(paths);
        let conf_path: PathBuf = themes_dir.parent().unwrap_or(&themes_dir).join("btop.conf");
        let config_status = HealthStatus::check_file(&conf_path, "btop.conf");

        if config_status.is_error() {
            return HealthStatus::error(
                "btop.conf missing",
                Some("Run `btop` once to generate default config"),
            );
        }

        if !theme.is_empty() {
            let content: String = fs::read_to_string(&conf_path).unwrap_or_default();
            let expected_line: String = format!("color_theme = \"{}\"", theme);

            if !content.contains(&expected_line) {
                return HealthStatus::Warning(format!(
                    "btop.conf is not using the current theme '{}'",
                    theme
                ));
            }

            let link: PathBuf = self.link_path(paths, theme);
            let link_status = HealthStatus::check_symlink(&link, "Theme file");
            if link_status.is_error() {
                return HealthStatus::error(
                    format!("Theme file `{theme}.theme` missing in btop themes folder"),
                    Some("Run `iris sync` or `iris health --fix` to restore the theme link"),
                );
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
        if !status.is_error() && !status.is_warning() {
            return task.log.action(
                &format!("Re-applied `{}` configuration", self.name().bold()),
                || self.apply(p, paths, templater, &mut task.as_quiet()),
            );
        }

        let mut fixed = false;
        if status.contains("missing in btop themes folder") {
            task.log.action("Restored `btop` theme symlink", || {
                let cache = self.cache_path(paths, &p.name);
                let link = self.link_path(paths, &p.name);
                self.ensure_symlink(&cache, &link)
            })?;
            fixed = true;
        }

        if status.contains("not using the current theme") {
            task.log
                .action("Updated btop.conf to use the correct theme", || {
                    let base_dir = self.resolve_config_directory(paths);
                    let conf_path = base_dir.parent().unwrap_or(&base_dir).join("btop.conf");
                    self.update_btop_conf(&conf_path, &p.name)
                })?;
            fixed = true;
        }

        if !fixed {
            task.log
                .action("Regenerated complete `btop` configuration", || {
                    self.apply(p, paths, templater, &mut task.as_quiet())
                })?;
        }

        Ok(())
    }
}

impl BtopGenerator {
    /// Update color_theme setting in btop.conf
    fn update_btop_conf(&self, path: &PathBuf, name: &str) -> Result<()> {
        if !path.exists() {
            return Ok(());
        }

        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read `btop` config: {}", path.display()))?;

        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        let mut updated: bool = false;
        let theme_line: String = format!("color_theme = \"{}\"", name);

        for line in lines.iter_mut() {
            if line.trim_start().starts_with("color_theme =") {
                *line = theme_line.clone();
                updated = true;
                break;
            }
        }

        if !updated {
            lines.push(theme_line);
        }

        fs::write(path, lines.join("\n"))
            .with_context(|| format!("Failed to update `btop` config: {}", path.display()))?;
        Ok(())
    }

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
                    "Failed to create `btop` cache directory: {}",
                    parent.display()
                )
            })?;
        }

        fs::write(&cache_file, content).with_context(|| {
            format!(
                "Failed to write `btop` theme file: {}",
                cache_file.display()
            )
        })?;
        Ok(cache_file)
    }

    fn ensure_symlink(&self, target: &Path, link: &Path) -> Result<()> {
        if link.exists() || link.is_symlink() {
            fs::remove_file(link).with_context(|| {
                format!(
                    "Failed to remove existing `btop` theme link: {}",
                    link.display()
                )
            })?;
        }

        if let Some(parent) = link.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "Failed to create parent directory for `btop` link: {}",
                    parent.display()
                )
            })?;
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            symlink(target, link).with_context(|| {
                format!(
                    "Failed to create `btop` symlink: {} -> {}",
                    target.display(),
                    link.display()
                )
            })?;
        }
        Ok(())
    }
}

/// Unit-tests for btop generator
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::tests::create_test_context;
    use tempdir::TempDir;

    #[test]
    fn should_return_btop_metadata() {
        let generator = BtopGenerator;
        assert_eq!(generator.name(), "btop");
        assert_eq!(generator.generator_type(), GeneratorType::System);
        assert_eq!(generator.target_file_name("iris-dark"), "iris-dark.theme");
    }

    #[test]
    fn should_build_valid_render_context() {
        let generator = BtopGenerator;
        let p = Palette::mock();
        let ctx = generator.build_render_context(&p);

        assert_eq!(ctx.get("bg").expect("bg missing").as_str().unwrap(), p.bg);
        assert_eq!(ctx.get("fg").expect("fg missing").as_str().unwrap(), p.fg);
        assert_eq!(
            ctx.get("keyword")
                .expect("keyword missing")
                .as_str()
                .unwrap(),
            p.keyword
        );

        assert!(ctx.contains_key("green"));
        assert!(ctx.contains_key("yellow"));
        assert!(ctx.contains_key("orange"));
        assert!(ctx.contains_key("type_name"));
        assert!(ctx.contains_key("theme_name"));
    }

    #[test]
    fn should_update_existing_line_or_append() {
        let generator = BtopGenerator;
        let temp_dir: TempDir = TempDir::new("btop_test").unwrap();
        let conf_path = temp_dir.path().join("btop.conf");

        fs::write(
            &conf_path,
            "theme_background = True\ncolor_theme = \"default\"\n",
        )
        .unwrap();
        generator.update_btop_conf(&conf_path, "new-theme").unwrap();
        let content = fs::read_to_string(&conf_path).unwrap();
        assert!(content.contains("color_theme = \"new-theme\""));
        assert!(!content.contains("color_theme = \"default\""));

        fs::write(&conf_path, "theme_background = True\n").unwrap();
        generator
            .update_btop_conf(&conf_path, "only-theme")
            .unwrap();
        let content = fs::read_to_string(&conf_path).unwrap();
        assert!(content.contains("color_theme = \"only-theme\""));
    }

    #[test]
    fn should_return_health_ok_for_btop() {
        let (_, mut ctx) = create_test_context();
        let generator = BtopGenerator;
        let p = Palette::mock();

        let mut task = ctx.log.step("Test", false).as_quiet();
        ctx.state.current_theme = p.name.clone();
        generator
            .apply(&p, &ctx.paths, &ctx.templater, &mut task)
            .unwrap();

        let btop_dir = generator
            .resolve_config_directory(&ctx.paths)
            .parent()
            .unwrap()
            .to_path_buf();
        fs::create_dir_all(&btop_dir).unwrap();
        let conf_path = btop_dir.join("btop.conf");

        let expected_line = format!("color_theme = \"{}\"", p.name);
        fs::write(
            &conf_path,
            format!("graph_symbol = \"braille\"\n{}", expected_line),
        )
        .unwrap();

        let status = generator.health_check(&ctx.paths, &p.name);
        assert!(status.is_ok(), "Expected Ok, got: {status}");
    }

    #[test]
    fn should_return_health_error_missing_conf_for_btop() {
        let (_, ctx) = create_test_context();
        let generator = BtopGenerator;
        let status = generator.health_check(&ctx.paths, &ctx.state.current_theme);

        assert!(
            status.is_error(),
            "Expected Error for missing btop.conf, got: {status}"
        );
        assert!(status.contains("btop.conf missing"));
    }

    #[test]
    fn should_return_health_warning_wrong_theme_in_conf_for_btop() {
        let (_, mut ctx) = create_test_context();
        let generator = BtopGenerator;
        let p = Palette::mock();

        let mut task = ctx.log.step("Test", false).as_quiet();
        ctx.state.current_theme = p.name.clone();
        generator
            .apply(&p, &ctx.paths, &ctx.templater, &mut task)
            .unwrap();

        let btop_dir = generator
            .resolve_config_directory(&ctx.paths)
            .parent()
            .unwrap()
            .to_path_buf();
        fs::create_dir_all(&btop_dir).unwrap();
        fs::write(btop_dir.join("btop.conf"), "color_theme = \"default\"").unwrap();

        let status = generator.health_check(&ctx.paths, &p.name);

        assert!(
            status.is_warning(),
            "Expected Warning for wrong theme line, got: {status}"
        );
        assert!(status.contains("not using the current theme"));
    }

    #[test]
    fn should_apply_theme_and_update_conf() {
        let (_, ctx) = create_test_context();
        let generator = BtopGenerator;
        let p = Palette::mock();

        let btop_dir = generator
            .resolve_config_directory(&ctx.paths)
            .parent()
            .unwrap()
            .to_path_buf();
        let btop_conf = btop_dir.join("btop.conf");

        fs::create_dir_all(&btop_dir).unwrap();
        fs::write(
            &btop_conf,
            "graph_symbol = \"braille\"\ncolor_theme = \"old-theme\"\n",
        )
        .unwrap();

        let mut task = ctx.log.step("Test", false).as_quiet();
        let result = generator.apply(&p, &ctx.paths, &ctx.templater, &mut task);
        assert!(result.is_ok());

        let cache_file = ctx.paths.generators.join("btop").join("test-theme.theme");
        assert!(cache_file.exists());

        let updated_content = fs::read_to_string(&btop_conf).unwrap();
        assert!(updated_content.contains(&format!("color_theme = \"{}\"", p.name)));
        assert!(updated_content.contains("graph_symbol = \"braille\""));
    }

    #[test]
    fn should_fix_broken_conf_for_btop() {
        let (tmp_dir, ctx) = create_test_context();
        let generator = BtopGenerator;
        let p = Palette::mock();
        let root = tmp_dir.path();

        let btop_dir = root.join(".config/btop");
        fs::create_dir_all(&btop_dir).unwrap();
        let conf_path = btop_dir.join("btop.conf");
        fs::write(
            &conf_path,
            "color_theme = \"wrong_theme\"\nother_setting = true",
        )
        .unwrap();

        let status = generator.health_check(&ctx.paths, &p.name);

        assert!(
            status.is_warning(),
            "Expected Warning for mismatched theme, got: {status}"
        );
        assert!(status.contains("not using the current theme"));

        let mut task = ctx.log.step("Test", false).as_quiet();
        generator
            .fix(&status, &p, &ctx.paths, &ctx.templater, &mut task)
            .expect("Fix failed");

        let content = fs::read_to_string(&conf_path).unwrap();
        assert!(content.contains(&format!("color_theme = \"{}\"", p.name)));
        assert!(content.contains("other_setting = true"));
    }

    #[test]
    fn should_fix_missing_theme_file_for_btop() {
        let (tmp_dir, mut ctx) = create_test_context();
        let generator = BtopGenerator;
        let p = Palette::mock();
        let root = tmp_dir.path();

        ctx.state.current_theme = p.name.clone();
        let btop_dir = root.join(".config/btop");
        fs::create_dir_all(btop_dir.join("themes")).unwrap();

        fs::write(
            btop_dir.join("btop.conf"),
            format!("color_theme = \"{}\"", p.name),
        )
        .unwrap();

        let mut task = ctx.log.step("Test", false).as_quiet();
        generator
            .apply(&p, &ctx.paths, &ctx.templater, &mut task)
            .unwrap();

        let link_path = generator.link_path(&ctx.paths, &p.name);
        assert!(link_path.exists());

        fs::remove_file(&link_path).unwrap();

        let status = generator.health_check(&ctx.paths, &p.name);
        assert!(
            status.is_error(),
            "Expected Error due to missing theme file, got: {status}"
        );

        generator
            .fix(&status, &p, &ctx.paths, &ctx.templater, &mut task)
            .expect("Fix failed");

        assert!(link_path.exists(), "Fix should restore the symlink");
        assert!(generator.health_check(&ctx.paths, &p.name).is_ok());
    }

    #[test]
    fn should_clear_generated_files_for_btop() {
        let (_, ctx) = create_test_context();
        let generator = BtopGenerator;

        let cache_dir = ctx.paths.generators.join(generator.name());
        fs::create_dir_all(&cache_dir).unwrap();

        let test_file = cache_dir.join("test.theme");
        fs::write(&test_file, "theme content").unwrap();

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
    fn should_remove_theme_for_btop() {
        let (_, ctx) = create_test_context();
        let generator = BtopGenerator;

        let cache_file = generator.cache_path(&ctx.paths, "test");
        let link_file = generator.theme_path(&ctx.paths, "test");

        fs::create_dir_all(cache_file.parent().unwrap()).unwrap();
        fs::create_dir_all(link_file.parent().unwrap()).unwrap();

        fs::write(&cache_file, "theme content").unwrap();
        fs::write(&link_file, "theme content").unwrap();

        assert!(
            cache_file.exists(),
            "Cache file should exist before removal"
        );
        assert!(link_file.exists(), "Theme file should exist before removal");

        generator.remove_theme(&ctx.paths, "test").unwrap();

        assert!(
            !cache_file.exists(),
            "remove_theme should delete the cache file"
        );
        assert!(
            !link_file.exists(),
            "remove_theme should delete the target theme file"
        );
    }
}
