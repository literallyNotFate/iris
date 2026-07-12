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

/// Config generator for herdr
pub struct HerdrGenerator;

impl Generator for HerdrGenerator {
    fn name(&self) -> &str {
        "herdr"
    }

    fn generator_type(&self) -> GeneratorType {
        GeneratorType::Multiplexer
    }

    fn target_file_name(&self, _theme: &str) -> String {
        "config.toml".into()
    }

    fn cache_path(&self, paths: &IrisPaths, theme: &str) -> PathBuf {
        paths
            .generators
            .join(self.name())
            .join(format!("{}_theme.toml", theme))
    }

    fn link_path(&self, paths: &IrisPaths, _theme: &str) -> PathBuf {
        self.resolve_config_directory(paths)
            .join(self.target_file_name(""))
    }

    fn active_link_path(&self, paths: &IrisPaths) -> Option<PathBuf> {
        Some(self.resolve_config_directory(paths).join("config.toml"))
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
            .context("Failed to render herdr theme block")?;

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

        c.insert("keyword", &theme.colors.keyword);
        c.insert("line_hl", &theme.colors.line_hl);
        c.insert("bg", &theme.colors.bg);
        c.insert("sel", &theme.colors.sel);
        c.insert("comment", &theme.colors.comment);
        c.insert("gutter_fg", &theme.colors.gutter_fg);
        c.insert("fg", &theme.colors.fg);
        c.insert("ansi", &theme.colors.ansi);

        c
    }

    fn health_check(&self, paths: &IrisPaths, theme: &str) -> HealthStatus {
        if !self.is_installed() {
            return HealthStatus::Warning("`herdr` binary not found".into());
        }

        let config_path: PathBuf = self.link_path(paths, "");
        let file_status = HealthStatus::check_file(&config_path, "config.toml");

        if file_status.is_error() {
            return HealthStatus::error(
                "config.toml not found",
                Some(format!("Create config at {}", config_path.display())),
            );
        }

        if !theme.is_empty() {
            let content: String = fs::read_to_string(&config_path).unwrap_or_default();
            let theme_lower: String = theme.to_lowercase();

            let expected_marker = format!("# iris_theme: {}", theme_lower);
            if !content.contains(&expected_marker) {
                return HealthStatus::Warning(format!(
                    "`herdr` is not using the current theme '{theme}'"
                ));
            }

            if !content.contains("[theme]") && !content.contains("[theme.") {
                return HealthStatus::error(
                    "Theme block '[theme]' missing in config",
                    Some("Run `iris sync` or `iris health --fix` to inject the theme block"),
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
                .context("Failed to clean up config.toml during clear")?;
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
                    .context("Failed to remove active theme from config.toml")?;
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

impl HerdrGenerator {
    fn clean_config_content(&self, content: &str) -> Vec<String> {
        let mut clean_lines: Vec<String> = Vec::new();
        let mut skip_block = false;

        for line in content.lines() {
            let trimmed: &str = line.trim();

            if trimmed.starts_with("# iris_theme:") {
                continue;
            }

            if trimmed.starts_with("[theme]") || trimmed.starts_with("[theme.") {
                skip_block = true;
                continue;
            }

            if skip_block && trimmed.starts_with('[') && !trimmed.starts_with("[theme.") {
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
                format!("Failed to read `herdr` config: {}", target_path.display())
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

        fs::write(write_path, final_content.trim())
            .with_context(|| format!("Failed to write `herdr` config: {}", write_path.display()))?;

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
                "Failed to write cleaned `herdr` config: {}",
                target_path.display()
            )
        })?;

        Ok(())
    }
}

/// Tests for herdr generator
#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::tests::mock_context;

    /// Unit-tests for herdr
    mod unit {
        use super::*;

        #[test]
        fn should_return_herdr_metadata() {
            let generator = HerdrGenerator;
            assert_eq!(generator.name(), "herdr");
            assert_eq!(generator.generator_type(), GeneratorType::Multiplexer);
            assert_eq!(generator.target_file_name("any"), "config.toml");
        }

        #[test]
        fn should_build_valid_render_context() {
            let generator = HerdrGenerator;
            let theme: Theme = Theme::mock();
            let ctx = generator.build_render_context(&theme);

            assert!(ctx.contains_key("keyword"));
            assert!(ctx.contains_key("line_hl"));
            assert!(ctx.contains_key("bg"));
            assert!(ctx.contains_key("sel"));
            assert!(ctx.contains_key("comment"));
            assert!(ctx.contains_key("gutter_fg"));
            assert!(ctx.contains_key("fg"));

            let ansi = ctx
                .get("ansi")
                .expect("ansi array missing")
                .as_array()
                .expect("ansi should be an array");
            assert!(ansi.len() >= 16);
        }

        #[test]
        fn should_clean_and_inject_correctly() {
            let (_iris_dir, ctx) = mock_context();

            let herdr_config_dir = ctx.paths.config.join("herdr");
            fs::create_dir_all(&herdr_config_dir).unwrap();
            let config_path = herdr_config_dir.join("config.toml");

            let initial_content = r##"# iris_theme: old_theme

        [kafka]
        brokers = ["localhost:9092"]

        [theme.custom]
        accent = "#000000"
        "##;
            fs::write(&config_path, initial_content).unwrap();

            let generator = HerdrGenerator;
            let theme: Theme = Theme::mock();
            let mock_theme_block = "[theme.custom]\naccent = \"#111111\"";

            generator
                .update_config_file(&config_path, &config_path, &theme, mock_theme_block)
                .unwrap();

            let result = fs::read_to_string(&config_path).unwrap();
            let theme_occurrences: Vec<_> = result.matches("iris_theme:").collect();

            assert_eq!(theme_occurrences.len(), 1);
            assert!(result.starts_with(&format!("# iris_theme: {}", theme.name.to_lowercase())));

            assert!(result.contains("[kafka]"));
            assert!(result.contains("brokers ="));
            assert!(!result.contains("#000000"));
            assert!(result.contains("#111111"));
        }

        #[test]
        fn should_apply_theme_for_herdr() {
            let (_iris_dir, ctx) = mock_context();

            let herdr_config_dir = ctx.paths.config.join("herdr");
            fs::create_dir_all(&herdr_config_dir).unwrap();
            let config_path = herdr_config_dir.join("config.toml");

            fs::write(&config_path, "[kafka]\nbrokers = []\n").unwrap();

            let generator = HerdrGenerator;
            let theme: Theme = Theme::mock();

            generator
                .update_config_file(&config_path, &config_path, &theme, "[theme.custom]\n")
                .unwrap();

            let final_content = fs::read_to_string(&config_path).unwrap();

            assert!(
                final_content.starts_with(&format!("# iris_theme: {}", theme.name.to_lowercase()))
            );
            assert!(final_content.contains("[kafka]"));
            assert!(final_content.contains("[theme.custom]"));
        }

        #[test]
        fn should_clear_generated_files_for_herdr() {
            let (_iris_dir, ctx) = mock_context();
            let generator = HerdrGenerator;
            let cache_dir = ctx.paths.generators.join(generator.name());

            fs::create_dir_all(&cache_dir).unwrap();
            fs::write(cache_dir.join("some_theme_style.toml"), "data").unwrap();

            generator.clear(&ctx.paths).unwrap();
            assert!(!cache_dir.exists());
        }

        #[test]
        fn should_remove_theme_for_herdr() {
            let (_iris_dir, ctx) = mock_context();

            let herdr_config_dir = ctx.paths.config.join("herdr");
            fs::create_dir_all(&herdr_config_dir).unwrap();
            let config_path = herdr_config_dir.join("config.toml");
            let theme_name = "test_theme";

            fs::write(
                &config_path,
                format!(
                    "# iris_theme: {}\n[kafka]\n[theme.custom]\naccent = \"#fb4934\"\n",
                    theme_name
                ),
            )
            .unwrap();

            let generator = HerdrGenerator;
            let cache_file = generator.cache_path(&ctx.paths, theme_name);
            fs::create_dir_all(cache_file.parent().unwrap()).unwrap();
            fs::write(&cache_file, "cache content").unwrap();

            generator.remove_styles_block(&config_path).unwrap();

            let final_content = fs::read_to_string(&config_path).unwrap();
            assert!(!final_content.contains("[theme.custom]"));
            assert!(!final_content.contains("accent"));
            assert!(!final_content.contains("iris_theme"));
        }
    }

    /// Integration tests for herdr
    mod integration {
        use super::*;
        use crate::skip_if_not_installed;

        #[test]
        fn should_return_health_ok_for_herdr() {
            skip_if_not_installed!(HerdrGenerator);

            let (_iris_dir, mut ctx) = mock_context();
            let generator = HerdrGenerator;
            let herdr_config_dir = generator.resolve_config_directory(&ctx.paths);
            fs::create_dir_all(&herdr_config_dir).unwrap();
            let config_path = generator.link_path(&ctx.paths, "");

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
        fn should_return_health_warning_wrong_theme_for_herdr() {
            skip_if_not_installed!(HerdrGenerator);

            let (_iris_dir, mut ctx) = mock_context();
            let generator = HerdrGenerator;
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
            skip_if_not_installed!(HerdrGenerator);

            let (_iris_dir, ctx) = mock_context();
            let generator = HerdrGenerator;

            let config_path = generator.link_path(&ctx.paths, "");
            if config_path.exists() {
                fs::remove_file(&config_path).unwrap();
            }

            let status = generator.health_check(&ctx.paths, "any");

            assert!(status.is_error(), "Expected Error, got: {status}");
            assert!(status.contains("not found"));
        }

        #[test]
        fn should_fix_wrong_theme_name_for_herdr() {
            skip_if_not_installed!(HerdrGenerator);

            let (_iris_dir, ctx) = mock_context();
            let generator = HerdrGenerator;
            let config_path = generator.link_path(&ctx.paths, "");

            if let Some(parent) = config_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }

            let old_complex_config = r##"# iris_theme: old-theme

        [kafka]
        brokers = ["localhost:9092"]

        [theme.custom]
        accent = "#fb4934"
        panel_bg = "#303030"
        "##;

            fs::write(&config_path, old_complex_config).unwrap();

            let theme: Theme = Theme::mock();
            let status = generator.health_check(&ctx.paths, &theme.name);

            assert!(status.is_warning(), "Expected Warning, got: {status}");

            let new_mock_styles = "[theme.custom]\naccent = \"#111111\"";
            generator
                .update_config_file(&config_path, &config_path, &theme, new_mock_styles)
                .unwrap();

            let content = fs::read_to_string(&config_path).unwrap();

            assert!(content.contains(&format!("# iris_theme: {}", theme.name.to_lowercase())));
            assert!(content.contains("[kafka]"));
            assert!(content.contains("brokers ="));
            assert!(!content.contains("#fb4934"));
            assert!(content.contains("#111111"));
            assert!(!content.contains("panel_bg"));
        }

        #[test]
        fn should_fix_missing_theme_marker_and_styles_for_herdr() {
            skip_if_not_installed!(HerdrGenerator);

            let (_iris_dir, ctx) = mock_context();
            let generator = HerdrGenerator;
            let config_path = generator.link_path(&ctx.paths, "");

            if let Some(parent) = config_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }

            fs::write(&config_path, "[kafka]\nbrokers = []\n").unwrap();

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
            assert!(content.contains("[kafka]"));
            assert!(generator.health_check(&ctx.paths, &theme.name).is_ok());
        }
    }
}
