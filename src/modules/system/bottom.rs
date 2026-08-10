use crate::{
    core::IrisEngine,
    infra::IrisPaths,
    models::{HealthStatus, Issue},
    modules::{Generator, GeneratorType, Strategy, traits::*},
};
use std::{fs, path::PathBuf};

/// Config generator for bottom
pub struct BottomGenerator;

impl Identifiable for BottomGenerator {
    fn name(&self) -> &'static str {
        "bottom"
    }

    fn generator_type(&self) -> GeneratorType {
        GeneratorType::System
    }

    fn is_installed(&self) -> bool {
        which::which("btm").is_ok()
    }
}

impl PathResolvable for BottomGenerator {
    fn target_file_name(&self, _theme: &str) -> String {
        "bottom.toml".into()
    }

    fn cache_path(&self, paths: &IrisPaths, theme: &str) -> PathBuf {
        paths
            .generators
            .join(self.name())
            .join(format!("{}_block.toml", theme))
    }

    fn link_path(&self, paths: &IrisPaths, _theme: &str) -> PathBuf {
        self.resolve_config_directory(paths)
            .join(self.target_file_name(""))
    }

    fn active_link_path(&self, paths: &IrisPaths) -> Option<PathBuf> {
        Some(self.resolve_config_directory(paths).join("bottom.toml"))
    }
}

impl Generator for BottomGenerator {
    fn strategy(&self) -> Strategy {
        Strategy::InjectBlock {
            file: "bottom.toml".to_string(),
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

impl BottomGenerator {
    pub fn remove_styles_block(&self, target_path: &PathBuf) -> anyhow::Result<()> {
        if !target_path.exists() {
            return Ok(());
        }

        let content: String = fs::read_to_string(target_path)?;
        let cleaned: String = crate::utils::replace_block(&content, self.name(), "");

        fs::write(target_path, cleaned.trim())?;
        Ok(())
    }
}

impl Diagnosable for BottomGenerator {
    fn health_check(&self, paths: &IrisPaths, theme: &str) -> HealthStatus {
        if !self.is_installed() {
            return HealthStatus::error(Issue::BinaryNotFound);
        }

        let config_path: PathBuf = self.link_path(paths, "");
        let file_status = HealthStatus::check_file(&config_path, Issue::ConfigMissing);
        if !file_status.is_ok() {
            return file_status;
        }

        let content: String = fs::read_to_string(&config_path).unwrap_or_default();

        let start_marker: String = format!("# [iris:begin:{}]", self.name());
        let end_marker: String = format!("# [iris:end:{}]", self.name());
        if !content.contains(&start_marker) || !content.contains(&end_marker) {
            return HealthStatus::warn(Issue::MarkerMissing);
        }

        if !theme.is_empty() {
            let theme_lower: String = theme.to_lowercase();
            let expected_marker: String = format!("# iris_theme: {}", theme_lower);
            if !content.contains(&expected_marker) {
                return HealthStatus::warn(Issue::MarkerMissing);
            }

            if !content.contains("[styles]") && !content.contains("[styles") {
                return HealthStatus::error(Issue::BlockMissing);
            }
        }

        HealthStatus::Ok
    }
}

impl Cleanable for BottomGenerator {
    fn cleanup(&self, paths: &IrisPaths) -> anyhow::Result<()> {
        default_cleanup(self, paths)
    }

    fn remove_theme(&self, paths: &IrisPaths, theme_name: &str) -> anyhow::Result<()> {
        let config_path: PathBuf = self.link_path(paths, "");
        if config_path.exists() {
            let content: String = fs::read_to_string(&config_path)?;
            let start_marker = format!("# [iris:begin:{}]", self.name());

            if content.contains(&start_marker) {
                self.remove_styles_block(&config_path)?;
            }
        }

        let cache_file: PathBuf = self.cache_path(paths, &theme_name.to_lowercase());
        if cache_file.exists() {
            fs::remove_file(cache_file)?;
        }

        Ok(())
    }

    fn cleanup_config(&self, config_path: &PathBuf) -> anyhow::Result<()> {
        self.remove_styles_block(config_path)
    }
}

impl Diffable for BottomGenerator {
    fn config_path(&self, paths: &IrisPaths) -> PathBuf {
        self.link_path(paths, "")
    }

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

/// Tests for bottom generator
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{core::IrisContext, models::Theme};

    /// Unit-tests for bottom
    mod unit {
        use super::*;

        #[test]
        fn should_return_metadata_for_bottom() {
            let generator = BottomGenerator;
            assert_eq!(generator.name(), "bottom");
            assert_eq!(generator.generator_type(), GeneratorType::System);
            assert_eq!(generator.target_file_name("any"), "bottom.toml");
        }

        #[test]
        fn should_build_valid_render_context_for_bottom() {
            let generator = BottomGenerator;
            let (_, ctx) = IrisContext::mock();
            let theme: Theme = Theme::mock();
            let ctx = ctx.engine(&theme).build_context(&generator).unwrap();

            assert_eq!(ctx.get("bg").unwrap().as_str().unwrap(), theme.colors.bg);
            assert_eq!(ctx.get("fg").unwrap().as_str().unwrap(), theme.colors.fg);
            assert!(ctx.get("ansi").unwrap().is_array());
            assert!(ctx.contains_key("operator"));
            assert!(ctx.contains_key("comment"));
            assert!(ctx.contains_key("gutter_fg"));
            assert!(ctx.contains_key("caret"));
        }

        #[test]
        fn should_clean_and_inject_correctly_for_bottom() {
            let (_iris_dir, ctx) = IrisContext::with_templates(vec![(
                "system/bottom",
                "[styles]\n\n[styles.widgets]\nbg_colour = \"{{ bg }}\"",
            )]);
            let btm_config_dir = ctx.paths.config.parent().unwrap().join("bottom");
            let config_path = btm_config_dir.join("bottom.toml");
            fs::create_dir_all(&btm_config_dir).unwrap();

            let initial_content = r##"[flags]
rate = 1000

# [iris:begin:bottom]
[styles]
bg_colour = "#000000"
# [iris:end:bottom]
"##;
            fs::write(&config_path, initial_content).unwrap();

            let generator = BottomGenerator;
            let theme: Theme = Theme::mock();

            let mut activity = ctx.log.step("Test", false).muted();
            ctx.engine(&theme)
                .execute_apply(&generator, &mut activity)
                .unwrap();

            let result = fs::read_to_string(&config_path).unwrap();

            assert!(result.contains("[flags]"));
            assert!(result.contains("# [iris:begin:bottom]"));
            assert!(result.contains("# [iris:end:bottom]"));
        }

        #[test]
        fn should_apply_theme_for_bottom() {
            let (_iris_dir, ctx) = IrisContext::with_templates(vec![(
                "system/bottom",
                "[styles]\n\n[styles.widgets]\nbg_colour = \"{{ bg }}\"",
            )]);

            let btm_config_dir = ctx.paths.config.parent().unwrap().join("bottom");
            let config_path = btm_config_dir.join("bottom.toml");
            fs::create_dir_all(&btm_config_dir).unwrap();
            fs::write(&config_path, "").unwrap();

            let generator = BottomGenerator;
            let theme: Theme = Theme::mock();

            let mut activity = ctx.log.step("Test", false).muted();
            ctx.engine(&theme)
                .execute_apply(&generator, &mut activity)
                .unwrap();

            let final_content = fs::read_to_string(&config_path).unwrap();
            assert!(!final_content.is_empty());
            assert!(final_content.contains("# [iris:begin:bottom]"));
            assert!(final_content.contains("# [iris:end:bottom]"));
            assert!(final_content.contains("[styles]"));
        }

        #[test]
        fn should_clear_generated_files_for_bottom() {
            let (_iris_dir, ctx) = IrisContext::mock();
            let generator = BottomGenerator;
            let cache_dir = ctx.paths.generators.join(generator.name());

            fs::create_dir_all(&cache_dir).unwrap();
            fs::write(cache_dir.join("some_theme_style.toml"), "data").unwrap();

            generator.cleanup(&ctx.paths).unwrap();
            assert!(!cache_dir.exists());
        }

        #[test]
        fn should_remove_theme_for_bottom() {
            let (_iris_dir, ctx) = IrisContext::mock();

            let btm_config_dir = ctx.paths.config.parent().unwrap().join("bottom");
            let config_path = btm_config_dir.join("bottom.toml");
            fs::create_dir_all(&btm_config_dir).unwrap();
            let theme_name = "test_theme";

            fs::write(&config_path,
                format!("[flags]\n# [iris:begin:bottom]\n[styles]\nbg_colour = \"#000000\"\n# [iris:end:bottom]\n"))
            .unwrap();

            let generator = BottomGenerator;
            let cache_file = generator.cache_path(&ctx.paths, theme_name);
            fs::create_dir_all(cache_file.parent().unwrap()).unwrap();
            fs::write(&cache_file, "cache content").unwrap();

            generator.remove_theme(&ctx.paths, theme_name).unwrap();
            assert!(config_path.exists());

            let final_content = fs::read_to_string(&config_path).unwrap();
            assert!(!final_content.contains("[styles]"));
            assert!(!final_content.contains("bg_colour"));
            assert!(final_content.contains("[flags]"));
            assert!(!cache_file.exists());
        }
    }

    /// Integration tests for bottom
    mod integration {
        use super::*;
        use crate::skip_if_not_installed;

        #[test]
        fn should_return_health_ok_for_bottom() {
            skip_if_not_installed!(BottomGenerator);

            let (_iris_dir, mut ctx) = IrisContext::with_templates(vec![(
                "system/bottom",
                "[styles]\n\n[styles.widgets]\nbg_colour = \"{{ bg }}\"",
            )]);
            let generator = BottomGenerator;
            let btm_config_dir = ctx.paths.config.join("bottom");
            fs::create_dir_all(&btm_config_dir).unwrap();

            let config_path = btm_config_dir.join("bottom.toml");
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
        fn should_return_health_warning_wrong_theme_for_bottom() {
            skip_if_not_installed!(BottomGenerator);

            let (_iris_dir, mut ctx) = IrisContext::mock();
            let generator = BottomGenerator;
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

            fs::write(&config_path, "[styles]\nbg_colour = \"#ffffff\"").unwrap();
            let status = generator.health_check(&ctx.paths, &theme.name);

            assert!(status.is_warning(), "Expected Warning, got: {status}");
            assert!(status.contains("Marker missing") || status.contains("missing"));
        }

        #[test]
        fn should_return_health_error_if_config_missing_for_bottom() {
            skip_if_not_installed!(BottomGenerator);

            let (_iris_dir, ctx) = IrisContext::mock();
            let generator = BottomGenerator;

            let config_path = generator.link_path(&ctx.paths, "");
            if config_path.exists() {
                fs::remove_file(&config_path).unwrap();
            }

            let status = generator.health_check(&ctx.paths, "any");

            assert!(status.is_error(), "Expected Error, got: {status}");
            assert!(status.contains("Configuration file missing"));
        }

        #[test]
        fn should_fix_wrong_theme_name_for_bottom() {
            skip_if_not_installed!(BottomGenerator);

            let (_iris_dir, ctx) = IrisContext::with_templates(vec![(
                "system/bottom",
                "[styles]\n\n[styles.widgets]\nbg_colour = \"{{ bg }}\"",
            )]);
            let generator = BottomGenerator;
            let config_path = generator.link_path(&ctx.paths, "");

            if let Some(parent) = config_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }

            let old_complex_config = r##"[flags]
        rate = 1000

        # [iris:begin:bottom]
        [styles]
        bg_colour = "#fb4934"
        # [iris:end:bottom]
        "##;

            fs::write(&config_path, old_complex_config).unwrap();

            let theme: Theme = Theme::mock();
            let status = generator.health_check(&ctx.paths, &theme.name);

            let mut activity = ctx.log.step("Test", false).muted();
            let engine = ctx.engine(&theme);
            engine
                .execute_fix(&generator, &status, &mut activity)
                .unwrap();

            let content = fs::read_to_string(&config_path).unwrap();

            assert!(content.contains("[flags]"));
            assert!(content.contains("rate = 1000"));
            assert!(!content.contains("#fb4934"));
            assert!(content.contains(&theme.colors.bg));
        }

        #[test]
        fn should_fix_missing_theme_marker_and_styles_for_bottom() {
            skip_if_not_installed!(BottomGenerator);

            let (_iris_dir, ctx) = IrisContext::with_templates(vec![(
                "system/bottom",
                "[styles]\n\n[styles.widgets]\nbg_colour = \"{{ bg }}\"",
            )]);
            let generator = BottomGenerator;
            let config_path = generator.link_path(&ctx.paths, "");

            if let Some(parent) = config_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }

            fs::write(&config_path, "[flags]\nrate = 1000\n").unwrap();

            let theme: Theme = Theme::mock();
            let status = generator.health_check(&ctx.paths, &theme.name);

            assert!(status.is_warning(), "Expected Warning, got: {status}");

            let mut activity = ctx.log.step("Fix", false);
            let engine = ctx.engine(&theme);
            engine
                .execute_fix(&generator, &status, &mut activity)
                .unwrap();

            let content = fs::read_to_string(&config_path).unwrap();
            assert!(content.contains("[flags]"));
            assert!(content.contains("# [iris:begin:bottom]"));
            assert!(content.contains("# [iris:end:bottom]"));
            assert!(generator.health_check(&ctx.paths, &theme.name).is_ok());
        }
    }
}
