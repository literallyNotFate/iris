use crate::{
    core::{InjectionPosition, IrisEngine},
    infra::IrisPaths,
    models::{HealthStatus, Issue},
    modules::{
        Generator, GeneratorType, Strategy,
        traits::{Cleanable, Diagnosable, Identifiable, PathResolvable},
    },
};
use std::{fs, path::PathBuf};

/// Config generator for wezterm terminal
pub struct WezTermGenerator;

impl Identifiable for WezTermGenerator {
    fn name(&self) -> &str {
        "wezterm"
    }

    fn generator_type(&self) -> GeneratorType {
        GeneratorType::Terminal
    }
}

impl PathResolvable for WezTermGenerator {
    fn target_file_name(&self, theme: &str) -> String {
        if theme.is_empty() {
            "current_theme.lua".into()
        } else {
            format!("{}.lua", theme.to_lowercase())
        }
    }

    fn link_path(&self, paths: &IrisPaths, _theme: &str) -> PathBuf {
        self.resolve_config_directory(paths)
            .join(self.target_file_name(""))
    }

    fn active_link_path(&self, paths: &IrisPaths) -> Option<PathBuf> {
        Some(
            self.resolve_config_directory(paths)
                .join("current_theme.lua"),
        )
    }
}

impl Generator for WezTermGenerator {
    fn strategy(&self) -> Strategy {
        Strategy::Symlink
    }

    fn pre_apply(&self, engine: &IrisEngine) -> anyhow::Result<()> {
        let config_path: PathBuf = self
            .resolve_config_directory(engine.paths)
            .join("wezterm.lua");

        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content: String = fs::read_to_string(&config_path).unwrap_or_default();
        if !config_path.exists() || content.trim().is_empty() || !content.contains("wezterm") {
            let default_config = concat!(
                "local has_iris, current_theme = pcall(require, \"current_theme\")\n",
                "local wezterm = require(\"wezterm\")\n",
                "local config = wezterm.config_builder()\n\n",
                "if has_iris and current_theme.colors then\n",
                "    config.colors = current_theme.colors\n",
                "end\n\n",
                "return config\n"
            );
            fs::write(&config_path, default_config)?;
            return Ok(());
        }

        engine.inject_line(
            &config_path,
            "local has_iris, current_theme = pcall(require, \"current_theme\")",
            InjectionPosition::Start,
        )?;
        let colors_block: &str = concat!(
            "if has_iris and current_theme.colors then\n",
            "    config.colors = current_theme.colors\n",
            "end"
        );

        let marker: &str = if content.contains("return config") {
            "return config"
        } else {
            "return"
        };

        engine.inject_line(
            &config_path,
            colors_block,
            InjectionPosition::Before(marker.to_string()),
        )
    }
}

impl Diagnosable for WezTermGenerator {
    fn health_check(&self, paths: &IrisPaths, theme: &str) -> HealthStatus {
        if !self.is_installed() {
            return HealthStatus::error(Issue::BinaryNotFound);
        }

        let wezterm_dir: PathBuf = self.resolve_config_directory(paths);
        let config_path: PathBuf = wezterm_dir.join("wezterm.lua");
        let link_path: PathBuf = self.link_path(paths, "");

        let config_status = HealthStatus::check_file(&config_path, Issue::ConfigMissing);
        if !config_status.is_ok() {
            return config_status;
        }

        let content: String = fs::read_to_string(&config_path).unwrap_or_default();
        if !content.contains("current_theme") || !content.contains("config.colors") {
            return HealthStatus::warn(Issue::ImportMissing);
        }

        let link_status = HealthStatus::check_symlink(&link_path, Issue::SymlinkInvalid);
        if !link_status.is_ok() {
            return link_status;
        }

        let expected_cache: PathBuf = self.cache_path(paths, theme);
        if let Ok(target) = fs::read_link(&link_path) {
            let abs_target: PathBuf = fs::canonicalize(&target).unwrap_or(target);
            let abs_expected: PathBuf = fs::canonicalize(&expected_cache).unwrap_or(expected_cache);

            if abs_target != abs_expected {
                return HealthStatus::warn(Issue::CacheMismatch);
            }
        }

        HealthStatus::Ok
    }
}

impl Cleanable for WezTermGenerator {
    fn cleanup(&self, paths: &IrisPaths) -> anyhow::Result<()> {
        crate::modules::traits::default_cleanup(self, paths)
    }

    fn remove_theme(&self, paths: &IrisPaths, theme_name: &str) -> anyhow::Result<()> {
        crate::modules::traits::default_remove(self, paths, theme_name)
    }
}

/// Tests for wezterm generator
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{core::IrisContext, models::Theme};

    /// Unit-tests for wezterm
    mod unit {
        use super::*;

        #[test]
        fn should_return_metadata_for_wezterm() {
            let generator = WezTermGenerator;
            assert_eq!(generator.name(), "wezterm");
            assert_eq!(generator.generator_type(), GeneratorType::Terminal);
            assert_eq!(generator.target_file_name("gruvbox"), "gruvbox.lua");
        }

        #[test]
        fn should_build_valid_render_context_for_wezterm() {
            let generator = WezTermGenerator;
            let (_, ctx) = IrisContext::mock();
            let theme: Theme = Theme::mock();
            let engine = ctx.engine(&theme);
            let ctx = engine.build_context(&generator).unwrap();

            assert_eq!(ctx.get("bg").unwrap().as_str().unwrap(), theme.colors.bg);
            assert_eq!(ctx.get("fg").unwrap().as_str().unwrap(), theme.colors.fg);
            assert!(ctx.get("ansi").unwrap().is_array());
        }

        #[test]
        fn should_apply_theme_for_wezterm() {
            let (_, ctx) = IrisContext::with_templates(vec![(
                "terminals/wezterm",
                "return {
              colors = {
                background = \"{{ bg }}\"
              }
            }",
            )]);
            let generator = WezTermGenerator;
            let theme: Theme = Theme::mock();

            let mut activity = ctx.log.step("Test", false).muted();
            let result = ctx.engine(&theme).execute_apply(&generator, &mut activity);
            assert!(result.is_ok(), "Failed to apply: {:?}", result.err());

            let cache_file = ctx.paths.generators.join("wezterm").join("test-theme.lua");
            assert!(cache_file.exists());

            let content = fs::read_to_string(cache_file).unwrap();
            assert!(content.contains("background ="));
        }

        #[test]
        fn should_clear_generated_files_for_wezterm() {
            let (_, ctx) = IrisContext::mock();
            let generator = WezTermGenerator;
            let cache_dir = ctx.paths.generators.join(generator.name());

            fs::create_dir_all(&cache_dir).unwrap();
            let file = cache_dir.join(generator.target_file_name(""));
            fs::write(&file, "test").unwrap();

            assert!(
                cache_dir.exists(),
                "Cache directory should exist before clearing"
            );

            generator.cleanup(&ctx.paths).unwrap();

            assert!(
                !cache_dir.exists(),
                "Clear should remove the entire generator cache directory"
            );
        }

        #[test]
        fn should_remove_theme_for_wezterm() {
            let (_, ctx) = IrisContext::mock();
            let generator = WezTermGenerator;

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

    /// Integration tests for wezterm
    mod integration {
        use super::*;
        use crate::skip_if_not_installed;

        #[test]
        fn should_return_health_ok_for_wezterm() {
            skip_if_not_installed!(WezTermGenerator);

            let (_, mut ctx) = IrisContext::mock();
            let generator = WezTermGenerator;
            let theme: Theme = Theme::mock();

            let mut activity = ctx.log.step("Test", false).muted();
            ctx.state.theme.current_theme = theme.name.clone();
            ctx.engine(&theme)
                .execute_apply(&generator, &mut activity)
                .unwrap();

            let config_path = generator
                .resolve_config_directory(&ctx.paths)
                .join("wezterm.lua");

            let valid_config = "local has_iris, current_theme = pcall(require, 'current_theme')\nconfig.colors = current_theme.colors";
            fs::write(&config_path, valid_config).unwrap();

            let status = generator.health_check(&ctx.paths, &theme.name);
            assert!(status.is_ok(), "Expected Ok, got: {status}");
        }

        #[test]
        fn should_return_health_error_missing_config_for_wezterm() {
            skip_if_not_installed!(WezTermGenerator);

            let (_, ctx) = IrisContext::mock();
            let generator = WezTermGenerator;
            let theme: Theme = Theme::mock();

            let status = generator.health_check(&ctx.paths, &theme.name);
            assert!(status.is_error(), "Expected Error, got: {status}");
            assert!(status.contains("Configuration file missing"));
        }

        #[test]
        fn should_return_health_warning_no_import_for_wezterm() {
            skip_if_not_installed!(WezTermGenerator);

            let (_, mut ctx) = IrisContext::mock();
            let generator = WezTermGenerator;
            let theme: Theme = Theme::mock();

            let mut activity = ctx.log.step("Test", false).muted();
            ctx.state.theme.current_theme = theme.name.clone();
            ctx.engine(&theme)
                .execute_apply(&generator, &mut activity)
                .unwrap();

            let config_path = generator
                .resolve_config_directory(&ctx.paths)
                .join("wezterm.lua");
            fs::write(&config_path, "local config = wezterm.config_builder()").unwrap();

            let status = generator.health_check(&ctx.paths, &theme.name);

            assert!(status.is_warning(), "Expected Warning, got: {status}");
            assert!(status.contains("Theme not imported"));
        }

        #[test]
        fn should_fix_inject_issue_for_wezterm() {
            skip_if_not_installed!(WezTermGenerator);

            let (_, ctx) = IrisContext::mock();
            let generator = WezTermGenerator;
            let theme: Theme = Theme::mock();

            let config_dir = generator.resolve_config_directory(&ctx.paths);
            fs::create_dir_all(&config_dir).unwrap();
            let config_path = config_dir.join("wezterm.lua");

            let mut activity = ctx.log.step("Test", false).muted();
            let engine = ctx.engine(&theme);
            engine.execute_apply(&generator, &mut activity).unwrap();

            fs::write(
                &config_path,
                "local wezterm = require(\"wezterm\")\nreturn config",
            )
            .unwrap();

            let status = generator.health_check(&ctx.paths, &theme.name);
            engine
                .execute_fix(&generator, &status, &mut activity)
                .unwrap();

            let content = fs::read_to_string(&config_path).unwrap();
            assert!(content.contains("current_theme"));
            assert!(content.contains("config.colors = current_theme.colors"));
            assert!(generator.health_check(&ctx.paths, &theme.name).is_ok());
        }

        #[test]
        fn should_fix_broken_link_for_wezterm() {
            skip_if_not_installed!(WezTermGenerator);

            let (_, mut ctx) = IrisContext::mock();
            let generator = WezTermGenerator;
            let theme: Theme = Theme::mock();
            ctx.state.theme.current_theme = theme.name.clone();

            let mut activity = ctx.log.step("Test", false).muted();
            let wezterm_dir = generator.resolve_config_directory(&ctx.paths);
            let config_path = wezterm_dir.join("wezterm.lua");
            fs::create_dir_all(&wezterm_dir).unwrap();
            let engine = ctx.engine(&theme);

            let valid_config = "local has_iris, current_theme = pcall(require, 'current_theme')\nconfig.colors = current_theme.colors";
            fs::write(&config_path, valid_config).unwrap();

            engine.execute_apply(&generator, &mut activity).unwrap();

            let link_path_empty = generator.link_path(&ctx.paths, "");
            if link_path_empty.exists() {
                fs::remove_file(&link_path_empty).unwrap();
            }

            let link_path_theme = generator.link_path(&ctx.paths, &theme.name);
            if link_path_theme.exists() && link_path_theme != link_path_empty {
                fs::remove_file(&link_path_theme).unwrap();
            }

            let status = generator.health_check(&ctx.paths, &theme.name);
            assert!(status.is_error());
            assert!(status.contains("Invalid symlink"));

            engine
                .execute_fix(&generator, &status, &mut activity)
                .unwrap();
            assert!(generator.health_check(&ctx.paths, &theme.name).is_ok());
        }
    }
}
