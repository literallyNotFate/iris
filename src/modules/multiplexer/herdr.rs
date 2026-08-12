use crate::{
    core::IrisEngine,
    infra::IrisPaths,
    models::{HealthStatus, Issue},
    modules::{Generator, GeneratorType, Strategy, traits::*},
};
use std::{fs, path::PathBuf};

/// Config generator for herdr
pub struct HerdrGenerator;

impl Identifiable for HerdrGenerator {
    fn name(&self) -> &'static str {
        "herdr"
    }

    fn generator_type(&self) -> GeneratorType {
        GeneratorType::Multiplexer
    }
}

impl PathResolvable for HerdrGenerator {
    fn base_file_name(&self) -> String {
        "config.toml".into()
    }

    fn cache_path(&self, paths: &IrisPaths, theme: &str) -> PathBuf {
        paths
            .generators
            .join(self.name())
            .join(format!("{}_block.toml", theme))
    }

    fn active_link_path(&self, paths: &IrisPaths) -> Option<PathBuf> {
        Some(self.config_path(paths))
    }
}

impl Generator for HerdrGenerator {
    fn strategy(&self) -> Strategy {
        Strategy::InjectBlock {
            file: "config.toml".to_string(),
        }
    }

    fn pre_apply(&self, engine: &IrisEngine) -> anyhow::Result<()> {
        let config_path: PathBuf = self.link_path(engine.paths, "");
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        engine.remove_marker(&config_path, "# iris_theme:")?;
        engine.inject_line(
            &config_path,
            &format!("# iris_theme: {}", engine.theme.name.to_lowercase()),
            crate::core::InjectionPosition::Start,
        )?;

        Ok(())
    }
}

impl Diagnosable for HerdrGenerator {
    fn health_check(&self, paths: &IrisPaths, theme: &str) -> HealthStatus {
        if !self.is_installed() {
            return HealthStatus::error(Issue::BinaryNotFound);
        }

        let config_path: PathBuf = self.link_path(paths, "");
        let config_status = HealthStatus::check_file(&config_path, Issue::ConfigMissing);
        if !config_status.is_ok() {
            return config_status;
        }

        let content: String = fs::read_to_string(&config_path).unwrap_or_default();

        let start_marker: String = format!("# [iris:begin:{}]", self.name());
        let end_marker: String = format!("# [iris:end:{}]", self.name());
        if !content.contains(&start_marker) || !content.contains(&end_marker) {
            return HealthStatus::warn(Issue::MarkerMissing);
        }

        if !theme.is_empty() {
            let theme_lower = theme.to_lowercase();
            let expected_marker = format!("# iris_theme: {}", theme_lower);
            if !content.contains(&expected_marker) {
                return HealthStatus::warn(Issue::MarkerMissing);
            }

            if !content.contains("[theme.custom]") {
                return HealthStatus::error(Issue::BlockMissing);
            }
        }

        HealthStatus::Ok
    }
}

impl Cleanable for HerdrGenerator {
    fn cleanup(&self, paths: &IrisPaths) -> anyhow::Result<()> {
        default_cleanup(self, paths)
    }

    fn remove_theme(&self, paths: &IrisPaths, theme_name: &str) -> anyhow::Result<()> {
        let config_path: PathBuf = self.link_path(paths, "");
        let theme_lower: String = theme_name.to_lowercase();

        if config_path.exists() {
            let content: String = fs::read_to_string(&config_path)?;
            let expected_theme_marker: String = format!("# iris_theme: {}", theme_lower);

            if content.contains(&expected_theme_marker) {
                self.remove_theme_block(&config_path)?;
            }
        }

        let cache_file: PathBuf = self.cache_path(paths, &theme_lower);
        if cache_file.exists() {
            fs::remove_file(cache_file)?;
        }

        Ok(())
    }

    fn cleanup_config(&self, config_path: &PathBuf) -> anyhow::Result<()> {
        self.remove_theme_block(config_path)
    }
}

impl HerdrGenerator {
    pub fn remove_theme_block(&self, target_path: &PathBuf) -> anyhow::Result<()> {
        if !target_path.exists() {
            return Ok(());
        }

        let content: String = fs::read_to_string(target_path)?;
        let cleaned: String = crate::utils::replace_block(&content, self.name(), "");

        fs::write(target_path, cleaned.trim())?;
        Ok(())
    }
}

impl Diffable for HerdrGenerator {
    fn ideal_content(&self, paths: &IrisPaths, theme: &str) -> anyhow::Result<String> {
        let cache_file: PathBuf = self.cache_path(paths, theme);
        if cache_file.exists() {
            let content = fs::read_to_string(cache_file)?;
            Ok(content)
        } else {
            Ok(String::new())
        }
    }
}

/// Tests for herdr generator
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{core::IrisContext, models::Theme};

    /// Unit-tests for herdr
    mod unit {
        use super::*;

        #[test]
        fn should_return_metadata_for_herdr() {
            let generator = HerdrGenerator;
            assert_eq!(generator.name(), "herdr");
            assert_eq!(generator.generator_type(), GeneratorType::Multiplexer);
            assert_eq!(generator.file_name("any"), "config.toml");
        }

        #[test]
        fn should_handle_path_resolution_for_herdr() {
            let (_temp_dir, ctx) = IrisContext::mock();
            let generator = HerdrGenerator;
            let theme = "tokyonight";

            let expected_config_dir = ctx.paths.config.parent().unwrap().join("herdr");
            assert_eq!(generator.config_dir(&ctx.paths), expected_config_dir);

            let expected_config_path = expected_config_dir.join("config.toml");
            assert_eq!(generator.config_path(&ctx.paths), expected_config_path);

            let expected_cache_path = ctx.paths.generators.join("herdr/tokyonight_block.toml");
            assert_eq!(generator.cache_path(&ctx.paths, theme), expected_cache_path);
            assert_eq!(generator.template_path(), "multiplexer/herdr");
        }

        #[test]
        fn should_build_valid_render_context_for_herdr() {
            let generator = HerdrGenerator;
            let (_, ctx) = IrisContext::mock();
            let theme: Theme = Theme::mock();
            let ctx = ctx.engine(&theme).build_context(&generator).unwrap();

            assert_eq!(ctx.get("bg").unwrap().as_str().unwrap(), theme.colors.bg);
            assert_eq!(ctx.get("fg").unwrap().as_str().unwrap(), theme.colors.fg);
            assert!(ctx.contains_key("keyword"));
            assert!(ctx.contains_key("line_hl"));
            assert!(ctx.contains_key("sel"));
            assert!(ctx.contains_key("comment"));
            assert!(ctx.contains_key("gutter_fg"));
            assert!(ctx.get("ansi").unwrap().is_array());
        }

        #[test]
        fn should_clean_and_inject_correctly_for_herdr() {
            let (_iris_dir, ctx) = IrisContext::with_templates(vec![(
                "multiplexer/herdr",
                "[theme.custom]\naccent = \"#000000\"",
            )]);
            let herdr_dir = ctx.paths.config.parent().unwrap().join("herdr");
            let config_path = herdr_dir.join("config.toml");
            fs::create_dir_all(&herdr_dir).unwrap();

            let initial_content = r##"# iris_theme: old_theme
[kafka]
brokers = ["localhost:9092"]

# [iris:begin:herdr]
[theme.custom]
accent = "#000000"
# [iris:end:herdr]
"##;
            fs::write(&config_path, initial_content).unwrap();

            let generator = HerdrGenerator;
            let theme: Theme = Theme::mock();
            let theme_lower = theme.name.to_lowercase();

            let mut activity = ctx.log.step("Test", false).muted();
            ctx.engine(&theme)
                .execute_apply(&generator, &mut activity)
                .unwrap();

            let result = fs::read_to_string(&config_path).unwrap();

            assert!(result.contains(&format!("# iris_theme: {}", theme_lower)));
            assert!(result.contains("[kafka]"));
            assert!(result.contains("# [iris:begin:herdr]"));
            assert!(result.contains("# [iris:end:herdr]"));
        }

        #[test]
        fn should_apply_theme_for_herdr() {
            let (_iris_dir, ctx) = IrisContext::with_templates(vec![(
                "multiplexer/herdr",
                "[theme.custom]\naccent = \"#000000\"",
            )]);

            let btm_config_dir = ctx.paths.config.parent().unwrap().join("herdr");
            let config_path = btm_config_dir.join("config.toml");
            fs::create_dir_all(&btm_config_dir).unwrap();
            fs::write(&config_path, "[kafka]\nbrokers = []\n").unwrap();

            let generator = HerdrGenerator;
            let theme: Theme = Theme::mock();

            let mut activity = ctx.log.step("Test", false).muted();
            ctx.engine(&theme)
                .execute_apply(&generator, &mut activity)
                .unwrap();

            let final_content = fs::read_to_string(&config_path).unwrap();

            assert!(
                final_content.starts_with(&format!("# iris_theme: {}", theme.name.to_lowercase()))
            );
            assert!(final_content.contains("[kafka]"));
            assert!(final_content.contains("[theme.custom]"));
            assert!(final_content.contains("# [iris:begin:herdr]"));
            assert!(final_content.contains("# [iris:end:herdr]"));
        }

        #[test]
        fn should_clear_generated_files_for_herdr() {
            let (_iris_dir, ctx) = IrisContext::mock();
            let generator = HerdrGenerator;
            let cache_dir = ctx.paths.generators.join(generator.name());

            fs::create_dir_all(&cache_dir).unwrap();
            fs::write(cache_dir.join("some_theme_style.toml"), "data").unwrap();

            generator.cleanup(&ctx.paths).unwrap();
            assert!(!cache_dir.exists());
        }

        #[test]
        fn should_remove_theme_for_herdr() {
            let (_iris_dir, ctx) = IrisContext::mock();

            let herdr_config_dir = ctx.paths.config.parent().unwrap().join("herdr");
            let config_path = herdr_config_dir.join("config.toml");
            fs::create_dir_all(&herdr_config_dir).unwrap();
            let theme_name = "test_theme";

            fs::write(
                &config_path,
                format!(
                    "# iris_theme: {}\n[kafka]\n# [iris:begin:herdr]\n[theme.custom]\naccent = \"#fb4934\"\n# [iris:end:herdr]\n",
                    theme_name
                ),
            )
            .unwrap();

            let generator = HerdrGenerator;
            let cache_file = generator.cache_path(&ctx.paths, theme_name);
            fs::create_dir_all(cache_file.parent().unwrap()).unwrap();
            fs::write(&cache_file, "cache content").unwrap();

            generator.remove_theme(&ctx.paths, theme_name).unwrap();

            assert!(config_path.exists());

            let final_content = fs::read_to_string(&config_path).unwrap();
            assert!(!final_content.contains("[theme.custom]"));
            assert!(!final_content.contains("accent"));
            assert!(!cache_file.exists());
        }
    }

    /// Integration tests for herdr
    mod integration {
        use super::*;
        use crate::skip_if_not_installed;

        #[test]
        fn should_return_health_ok_for_herdr() {
            skip_if_not_installed!(HerdrGenerator);

            let (_iris_dir, mut ctx) = IrisContext::with_templates(vec![(
                "multiplexer/herdr",
                "[theme.custom]\naccent = \"#000000\"",
            )]);
            let generator = HerdrGenerator;

            let config_path = generator.link_path(&ctx.paths, "");
            if let Some(parent) = config_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&config_path, "").unwrap();

            let theme: Theme = Theme::mock();
            ctx.state.theme.current_theme = theme.name.clone();
            let mut activity = ctx.log.step("Test", false).muted();

            ctx.engine(&theme)
                .execute_apply(&generator, &mut activity)
                .unwrap();

            let status = generator.health_check(&ctx.paths, &theme.name);
            assert!(status.is_ok(), "Expected Ok, got: {status}");
        }

        #[test]
        fn should_return_health_warning_wrong_theme_for_herdr() {
            skip_if_not_installed!(HerdrGenerator);

            let (_iris_dir, mut ctx) = IrisContext::mock();
            let generator = HerdrGenerator;
            let config_path = generator.link_path(&ctx.paths, "");
            if let Some(parent) = config_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }

            let theme: Theme = Theme::mock();
            ctx.state.theme.current_theme = theme.name.clone();
            let mut activity = ctx.log.step("Test", false).muted();

            ctx.engine(&theme)
                .execute_apply(&generator, &mut activity)
                .unwrap();

            let content = fs::read_to_string(&config_path).unwrap();
            let corrupted = content.replace(
                &format!("# [iris:begin:{}]", generator.name()),
                "# [iris:begin:corrupted]",
            );
            fs::write(&config_path, corrupted).unwrap();

            let status = generator.health_check(&ctx.paths, &theme.name);

            assert!(status.is_warning(), "Expected Warning, got: {status}");
            assert!(status.contains("Marker missing") || status.contains("missing"));
        }

        #[test]
        fn should_return_health_error_if_config_missing_for_herdr() {
            skip_if_not_installed!(HerdrGenerator);

            let (_iris_dir, ctx) = IrisContext::mock();
            let generator = HerdrGenerator;

            let config_path = generator.link_path(&ctx.paths, "");
            if config_path.exists() {
                fs::remove_file(&config_path).unwrap();
            }

            let status = generator.health_check(&ctx.paths, "any");

            assert!(status.is_error(), "Expected Error, got: {status}");
            assert!(status.contains("Configuration file missing"));
        }

        #[test]
        fn should_fix_wrong_theme_name_for_herdr() {
            skip_if_not_installed!(HerdrGenerator);

            let (_iris_dir, mut ctx) = IrisContext::with_templates(vec![(
                "multiplexer/herdr",
                "[theme.custom]\naccent = \"#000000\"",
            )]);
            let generator = HerdrGenerator;
            let config_path = generator.link_path(&ctx.paths, "");

            if let Some(parent) = config_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }

            let old_complex_config = r##"# iris_theme: old-theme

        [kafka]
        brokers = ["localhost:9092"]

        # [iris:begin:herdr]
        [theme.custom]
        accent = "#fb4934"
        panel_bg = "#303030"
        # [iris:end:herdr]
        "##;
            fs::write(&config_path, old_complex_config).unwrap();

            let theme: Theme = Theme::mock();
            ctx.state.theme.current_theme = theme.name.clone();

            let status = generator.health_check(&ctx.paths, &theme.name);
            assert!(
                status.is_warning() || status.is_error(),
                "Expected Warn/Err, got: {status}"
            );

            let mut activity = ctx.log.step("Test", false).muted();
            let engine = ctx.engine(&theme);
            engine
                .execute_fix(&generator, &status, &mut activity)
                .unwrap();

            let content = fs::read_to_string(&config_path).unwrap();

            assert!(content.contains("[kafka]"));
            assert!(content.contains("brokers ="));
            assert!(!content.contains("#fb4934"));
            assert!(content.contains("#000000"));
            assert!(!content.contains("panel_bg"));
        }

        #[test]
        fn should_fix_missing_theme_marker_and_styles_for_herdr() {
            skip_if_not_installed!(HerdrGenerator);

            let (_iris_dir, ctx) = IrisContext::with_templates(vec![(
                "multiplexer/herdr",
                "[theme.custom]\naccent = \"#000000\"",
            )]);
            let generator = HerdrGenerator;
            let config_path = generator.link_path(&ctx.paths, "");

            if let Some(parent) = config_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }

            fs::write(&config_path, "[kafka]\nbrokers = []\n").unwrap();

            let theme: Theme = Theme::mock();
            let status = generator.health_check(&ctx.paths, &theme.name);

            assert!(status.is_warning(), "Expected Warning, got: {status}");
            assert!(status.contains("Marker missing") || status.contains("missing"));

            let mut activity = ctx.log.step("Fix", false);
            let engine = ctx.engine(&theme);
            engine
                .execute_fix(&generator, &status, &mut activity)
                .unwrap();

            let content = fs::read_to_string(&config_path).unwrap();
            assert!(content.contains("[kafka]"));
            assert!(content.contains("# [iris:begin:herdr]"));
            assert!(content.contains("# [iris:end:herdr]"));
            assert!(generator.health_check(&ctx.paths, &theme.name).is_ok());
        }
    }
}
