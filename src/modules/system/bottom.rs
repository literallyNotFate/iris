use crate::{
    core::{IrisPaths, Templater},
    guards::FsRollbackGuard,
    log::Activity,
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

/// Config generator for bottom
pub struct BottomGenerator;

impl Generator for BottomGenerator {
    fn name(&self) -> &str {
        "bottom"
    }

    fn generator_type(&self) -> GeneratorType {
        GeneratorType::System
    }

    fn target_file_name(&self, _theme: &str) -> String {
        "bottom.toml".into()
    }

    fn cache_path(&self, paths: &IrisPaths, theme: &str) -> PathBuf {
        paths
            .generators
            .join(self.name())
            .join(format!("{}_style.toml", theme))
    }

    fn link_path(&self, paths: &IrisPaths, _theme: &str) -> PathBuf {
        self.resolve_config_directory(paths)
            .join(self.target_file_name(""))
    }

    fn active_link_path(&self, paths: &IrisPaths) -> Option<PathBuf> {
        Some(self.resolve_config_directory(paths).join("bottom.toml"))
    }

    fn apply(
        &self,
        theme: &Theme,
        paths: &IrisPaths,
        templater: &Templater,
        task: &mut Activity,
    ) -> Result<()> {
        task.info(&format!(
            "Generating {} theme for {}...",
            theme.name.yellow(),
            self.name().bold().cyan()
        ));

        let cache_file: PathBuf = self.cache_path(paths, &theme.name.to_lowercase());
        let link_path: PathBuf = self.link_path(paths, &theme.name.to_lowercase());
        let backup_path: PathBuf = link_path.with_extension("toml.bak");

        let render_ctx = self.build_render_context(theme);
        let styles_block: String = templater
            .render(&self.template_path(), &render_ctx)
            .context("Failed to render bottom styles block")?;

        if let Some(parent) = cache_file.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&cache_file, &styles_block).with_context(|| {
            format!("Failed to write palette cache to {}", cache_file.display())
        })?;

        let rollback_guard = FsRollbackGuard::new(link_path.clone(), backup_path);

        self.update_config_file(&link_path, &link_path, theme, &styles_block)?;
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
        c.insert("bg", &theme.colors.bg);
        c.insert("fg", &theme.colors.fg);
        c.insert("sel", &theme.colors.sel);
        c.insert("line_hl", &theme.colors.line_hl);
        c.insert("gutter_fg", &theme.colors.gutter_fg);
        c.insert("comment", &theme.colors.comment);
        c.insert("keyword", &theme.colors.keyword);
        c.insert("func", &theme.colors.func);
        c.insert("type_name", &theme.colors.type_name);
        c.insert("caret", &theme.colors.caret);
        c.insert("number", &theme.colors.number);
        c.insert("operator", &theme.colors.operator);
        c.insert("ansi", &theme.colors.ansi);
        c
    }

    fn is_installed(&self) -> bool {
        which::which("btm").is_ok()
    }

    fn health_check(&self, paths: &IrisPaths, theme: &str) -> HealthStatus {
        if !self.is_installed() {
            return HealthStatus::Warning("`btm` binary not found".into());
        }

        let config_path: PathBuf = self.link_path(paths, "");
        let file_status = HealthStatus::check_file(&config_path, "bottom.toml");

        if file_status.is_error() {
            return HealthStatus::error(
                "bottom.toml not found",
                Some(format!("Create config at {}", config_path.display())),
            );
        }

        if !theme.is_empty() {
            let content: String = fs::read_to_string(&config_path).unwrap_or_default();
            let theme_lower: String = theme.to_lowercase();

            let expected_marker = format!("# iris_theme: {}", theme_lower);
            if !content.contains(&expected_marker) {
                return HealthStatus::Warning(format!(
                    "`bottom` is not using the current theme '{theme}'"
                ));
            }

            if !content.contains("[styles]") && !content.contains("[styles.") {
                return HealthStatus::error(
                    "Styles block '[styles]' missing in config",
                    Some("Run `iris sync` or `iris health --fix` to inject the styles block"),
                );
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
        if status.is_error() || status.is_warning() {
            return task.log.action(
                &format!("Re-applied `{}` configuration", self.name().bold()),
                || self.apply(theme, paths, templater, &mut task.muted()),
            );
        }

        Ok(())
    }

    fn clear(&self, paths: &IrisPaths) -> Result<()> {
        let name: &str = self.name();
        let config_path: PathBuf = self.link_path(paths, "");

        if config_path.exists() {
            let backup_path: PathBuf = config_path.with_extension("toml.bak");
            let rollback_guard = FsRollbackGuard::new(config_path.clone(), backup_path);

            self.remove_styles_block(&config_path)
                .context("Failed to clean up bottom.toml during clear")?;
            rollback_guard.commit();
        }

        let gen_cache_dir: PathBuf = paths.generators.join(name);
        if gen_cache_dir.exists() {
            fs::remove_dir_all(&gen_cache_dir).with_context(|| {
                format!(
                    "Failed to remove generator directory for {}: {}",
                    name,
                    utils::pretty_path(&gen_cache_dir)
                )
            })?;
        }

        Ok(())
    }

    fn remove_theme(&self, paths: &IrisPaths, theme_name: &str) -> Result<()> {
        let theme_name_lower: String = theme_name.to_lowercase();
        let config_path: PathBuf = self.link_path(paths, "");

        if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            let active_marker: String = format!("# iris_theme: {}", theme_name_lower);

            if content.contains(&active_marker) {
                self.remove_styles_block(&config_path)
                    .context("Failed to remove active theme from bottom.toml")?;
            }
        }

        let theme_cache_file: PathBuf = self.cache_path(paths, &theme_name_lower);
        if theme_cache_file.exists() {
            fs::remove_file(&theme_cache_file).with_context(|| {
                format!(
                    "Failed to remove theme cache: {}",
                    theme_cache_file.display()
                )
            })?;
        }
        Ok(())
    }
}

impl BottomGenerator {
    fn clean_config_content(&self, content: &str) -> Vec<String> {
        let mut clean_lines: Vec<String> = Vec::new();
        let mut skip_block = false;

        for line in content.lines() {
            let trimmed: &str = line.trim();

            if trimmed.starts_with("# iris_theme:") {
                continue;
            }

            if trimmed.starts_with("[styles]") || trimmed.starts_with("[styles.") {
                skip_block = true;
                continue;
            }

            if skip_block && trimmed.starts_with('[') && !trimmed.starts_with("[styles.") {
                skip_block = false;
            }

            if !skip_block {
                clean_lines.push(line.to_string());
            }
        }

        clean_lines
    }

    fn update_config_file(
        &self,
        target_path: &Path,
        write_path: &Path,
        theme: &Theme,
        palette_block: &str,
    ) -> Result<()> {
        let content = if target_path.exists() {
            fs::read_to_string(target_path).with_context(|| {
                format!("Failed to read `bottom` config: {}", target_path.display())
            })?
        } else {
            String::new()
        };

        let clean_lines = self.clean_config_content(&content);
        let mut final_content = String::new();

        final_content.push_str(&format!("# iris_theme: {}\n\n", theme.name.to_lowercase()));

        let body = clean_lines.join("\n").trim().to_string();
        if !body.is_empty() {
            final_content.push_str(&body);
            final_content.push_str("\n\n");
        }

        final_content.push_str(palette_block);
        if let Some(parent) = write_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(write_path, final_content.trim()).with_context(|| {
            format!("Failed to write `bottom` config: {}", write_path.display())
        })?;

        Ok(())
    }

    pub fn remove_styles_block(&self, target_path: &Path) -> Result<()> {
        if !target_path.exists() {
            return Ok(());
        }

        let content: String = fs::read_to_string(target_path)?;
        let clean_lines: Vec<String> = self.clean_config_content(&content);

        fs::write(target_path, clean_lines.join("\n").trim()).with_context(|| {
            format!(
                "Failed to write cleaned `bottom` config: {}",
                target_path.display()
            )
        })?;

        Ok(())
    }
}

/// Tests for bottom generator
#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::tests::mock_context;

    /// Unit-tests for bottom
    mod unit {
        use super::*;

        #[test]
        fn should_return_bottom_metadata() {
            let generator = BottomGenerator;
            assert_eq!(generator.name(), "bottom");
            assert_eq!(generator.generator_type(), GeneratorType::System);
            assert_eq!(generator.target_file_name("any"), "bottom.toml");
        }

        #[test]
        fn should_build_valid_render_context() {
            let generator = BottomGenerator;
            let theme: Theme = Theme::mock();
            let ctx = generator.build_render_context(&theme);

            let ansi = ctx
                .get("ansi")
                .expect("ansi array missing")
                .as_array()
                .expect("ansi should be an array");
            assert!(ansi.len() >= 16);
            assert!(ctx.contains_key("operator"));
            assert!(ctx.contains_key("comment"));
            assert!(ctx.contains_key("gutter_fg"));
            assert!(ctx.contains_key("caret"));
        }

        #[test]
        fn should_clean_and_inject_correctly() {
            let (_iris_dir, ctx) = mock_context();

            let btm_config_dir = ctx.paths.config.join("bottom");
            fs::create_dir_all(&btm_config_dir).unwrap();
            let config_path = btm_config_dir.join("bottom.toml");

            let initial_content = r##"# iris_theme: old_theme

        [flags]
        rate = 1000

        [styles]
        bg_colour = "#000000"
        "##;
            fs::write(&config_path, initial_content).unwrap();

            let generator = BottomGenerator;
            let theme: Theme = Theme::mock();
            let mock_styles_block = "[styles]\nbg_colour = \"#111111\"";

            generator
                .update_config_file(&config_path, &config_path, &theme, mock_styles_block)
                .unwrap();

            let result = fs::read_to_string(&config_path).unwrap();
            let theme_occurrences: Vec<_> = result.matches("iris_theme:").collect();

            assert_eq!(theme_occurrences.len(), 1);
            assert!(result.starts_with(&format!("# iris_theme: {}", theme.name.to_lowercase())));

            assert!(result.contains("[flags]"));
            assert!(result.contains("rate = 1000"));
            assert!(!result.contains("#000000"));
            assert!(result.contains("#111111"));
        }

        #[test]
        fn should_apply_theme_for_bottom() {
            let (_iris_dir, ctx) = mock_context();

            let btm_config_dir = ctx.paths.config.join("bottom");
            fs::create_dir_all(&btm_config_dir).unwrap();
            let config_path = btm_config_dir.join("bottom.toml");

            fs::write(&config_path, "[flags]\nrate = 1000\n").unwrap();

            let generator = BottomGenerator;
            let theme: Theme = Theme::mock();

            generator
                .update_config_file(&config_path, &config_path, &theme, "[styles]\n")
                .unwrap();

            let final_content = fs::read_to_string(&config_path).unwrap();

            assert!(
                final_content.starts_with(&format!("# iris_theme: {}", theme.name.to_lowercase()))
            );
            assert!(final_content.contains("[flags]"));
            assert!(final_content.contains("[styles]"));
        }

        #[test]
        fn should_clear_generated_files_for_bottom() {
            let (_iris_dir, ctx) = mock_context();
            let generator = BottomGenerator;
            let cache_dir = ctx.paths.generators.join(generator.name());

            fs::create_dir_all(&cache_dir).unwrap();
            fs::write(cache_dir.join("some_theme_style.toml"), "data").unwrap();

            generator.clear(&ctx.paths).unwrap();
            assert!(!cache_dir.exists());
        }

        #[test]
        fn should_remove_theme_for_bottom() {
            let (_iris_dir, ctx) = mock_context();

            let btm_config_dir = ctx.paths.config.join("bottom");
            fs::create_dir_all(&btm_config_dir).unwrap();
            let config_path = btm_config_dir.join("bottom.toml");
            let theme_name = "test_theme";

            fs::write(
                &config_path,
                format!(
                    "# iris_theme: {}\n[flags]\n[styles]\nbg_colour = \"#000000\"\n",
                    theme_name
                ),
            )
            .unwrap();

            let generator = BottomGenerator;
            let cache_file = generator.cache_path(&ctx.paths, theme_name);
            fs::create_dir_all(cache_file.parent().unwrap()).unwrap();
            fs::write(&cache_file, "cache content").unwrap();

            generator.remove_styles_block(&config_path).unwrap();

            let final_content = fs::read_to_string(&config_path).unwrap();
            assert!(!final_content.contains("[styles]"));
            assert!(!final_content.contains("bg_colour"));
            assert!(!final_content.contains("iris_theme"));
        }
    }

    /// Integration tests for bottom
    mod integration {
        use super::*;
        use crate::skip_if_not_installed;

        #[test]
        fn should_return_health_ok_for_bottom() {
            skip_if_not_installed!(BottomGenerator);

            let (_iris_dir, mut ctx) = mock_context();
            let generator = BottomGenerator;
            let btm_config_dir = ctx.paths.config.join("bottom");
            fs::create_dir_all(&btm_config_dir).unwrap();
            let config_path = btm_config_dir.join("bottom.toml");

            fs::write(&config_path, "").unwrap();

            let theme: Theme = Theme::mock();
            ctx.state.theme.current_theme = theme.name.clone();
            let mut task = ctx.log.step("Test", false).muted();

            generator
                .apply(&theme, &ctx.paths, &ctx.templater, &mut task)
                .unwrap();

            let status = generator.health_check(&ctx.paths, &theme.name);
            assert!(status.is_ok(), "Expected Ok, got: {status}");
        }

        #[test]
        fn should_return_health_warning_wrong_theme_for_bottom() {
            skip_if_not_installed!(BottomGenerator);

            let (_iris_dir, mut ctx) = mock_context();
            let generator = BottomGenerator;
            let config_path = generator.link_path(&ctx.paths, "");
            if let Some(parent) = config_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }

            let theme: Theme = Theme::mock();
            ctx.state.theme.current_theme = theme.name.clone();
            let mut task = ctx.log.step("Test", false).muted();

            generator
                .apply(&theme, &ctx.paths, &ctx.templater, &mut task)
                .unwrap();

            let content = fs::read_to_string(&config_path).unwrap();
            let corrupted = content.replace(
                &format!("# iris_theme: {}", theme.name.to_lowercase()),
                "# iris_theme: wrong_theme",
            );
            fs::write(&config_path, corrupted).unwrap();

            let status = generator.health_check(&ctx.paths, &theme.name);

            assert!(status.is_warning(), "Expected Warning, got: {status}");
            assert!(status.contains("not using the current theme"));
        }

        #[test]
        fn should_return_health_error_if_config_missing() {
            skip_if_not_installed!(BottomGenerator);

            let (_iris_dir, ctx) = mock_context();
            let generator = BottomGenerator;

            let config_path = generator.link_path(&ctx.paths, "");
            if config_path.exists() {
                fs::remove_file(&config_path).unwrap();
            }

            let status = generator.health_check(&ctx.paths, "any");

            assert!(status.is_error(), "Expected Error, got: {status}");
            assert!(status.contains("not found"));
        }

        #[test]
        fn should_fix_wrong_theme_name_for_bottom() {
            skip_if_not_installed!(BottomGenerator);

            let (_iris_dir, ctx) = mock_context();
            let generator = BottomGenerator;
            let config_path = generator.link_path(&ctx.paths, "");

            if let Some(parent) = config_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }

            let old_complex_config = r##"# iris_theme: old-theme

        [flags]
        rate = 1000

        [styles]
        [styles.widgets]
        bg_colour = "#292522"
        widget_title = { colour = "#ebc06d", bold = true }

        [styles.cpu]
        all_entry_colour = "#ece1d7"
        cpu_core_colours = ["#7F91B2", "#78997A"]
        "##;

            fs::write(&config_path, old_complex_config).unwrap();

            let theme: Theme = Theme::mock();
            let status = generator.health_check(&ctx.paths, &theme.name);

            assert!(status.is_warning(), "Expected Warning, got: {status}");

            let new_mock_styles = "[styles]\n[styles.widgets]\nbg_colour = \"#111111\"";
            generator
                .update_config_file(&config_path, &config_path, &theme, new_mock_styles)
                .unwrap();

            let content = fs::read_to_string(&config_path).unwrap();

            assert!(content.contains(&format!("# iris_theme: {}", theme.name.to_lowercase())));
            assert!(content.contains("[flags]"));
            assert!(content.contains("rate = 1000"));
            assert!(!content.contains("#292522"));
            assert!(content.contains("#111111"));
            assert!(!content.contains("[styles.cpu]"));
            assert!(!content.contains("#ece1d7"));
        }

        #[test]
        fn should_fix_missing_theme_marker_and_styles_for_bottom() {
            skip_if_not_installed!(BottomGenerator);

            let (_iris_dir, ctx) = mock_context();
            let generator = BottomGenerator;
            let config_path = generator.link_path(&ctx.paths, "");

            if let Some(parent) = config_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }

            fs::write(&config_path, "[flags]\nrate = 1000\n").unwrap();

            let theme: Theme = Theme::mock();
            let status = generator.health_check(&ctx.paths, &theme.name);

            assert!(status.is_warning(), "Expected Warning, got: {status}");
            assert!(status.contains("not using the current theme"));

            let mut task = ctx.log.step("Fix", false);
            generator
                .fix(&status, &theme, &ctx.paths, &ctx.templater, &mut task)
                .unwrap();

            let content = fs::read_to_string(&config_path).unwrap();
            assert!(content.contains(&format!("# iris_theme: {}", theme.name.to_lowercase())));
            assert!(content.contains("[flags]"));
            assert!(generator.health_check(&ctx.paths, &theme.name).is_ok());
        }
    }
}
