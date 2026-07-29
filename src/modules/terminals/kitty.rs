use crate::{
    core::{InjectionPosition, IrisEngine},
    infra::IrisPaths,
    models::{HealthStatus, Issue},
    modules::{Cleanable, Generator, GeneratorType, Strategy},
};
use std::{fs, path::PathBuf};

/// Config generator for kitty terminal
pub struct KittyGenerator;

impl Generator for KittyGenerator {
    fn name(&self) -> &str {
        "kitty"
    }

    fn generator_type(&self) -> GeneratorType {
        GeneratorType::Terminal
    }

    fn strategy(&self) -> Strategy {
        Strategy::Symlink
    }

    fn target_file_name(&self, theme: &str) -> String {
        if theme.is_empty() {
            "current_theme.conf".into()
        } else {
            format!("{}.conf", theme.to_lowercase())
        }
    }

    fn link_path(&self, paths: &IrisPaths, _theme: &str) -> PathBuf {
        self.resolve_config_directory(paths)
            .join(self.target_file_name(""))
    }

    fn active_link_path(&self, paths: &IrisPaths) -> Option<PathBuf> {
        Some(
            self.resolve_config_directory(paths)
                .join("current_theme.conf"),
        )
    }

    fn pre_apply(&self, engine: &IrisEngine) -> anyhow::Result<()> {
        let config_path: PathBuf = self
            .resolve_config_directory(engine.paths)
            .join("kitty.conf");
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        engine.inject_line(
            &config_path,
            &format!("include {}", self.target_file_name("")),
            InjectionPosition::Start,
        )
    }

    fn health_check(&self, paths: &IrisPaths, theme: &str) -> HealthStatus {
        if !self.is_installed() {
            return HealthStatus::error(Issue::BinaryNotFound);
        }

        let kitty_dir: PathBuf = self.resolve_config_directory(paths);
        let config_path: PathBuf = kitty_dir.join("kitty.conf");
        let link_path: PathBuf = self.link_path(paths, "");

        let config_status = HealthStatus::check_file(&config_path, Issue::ConfigMissing);
        if !config_status.is_ok() {
            return config_status;
        }

        let content: String = fs::read_to_string(&config_path).unwrap_or_default();
        let import_line: String = format!("include {}", self.target_file_name(""));
        if !content.contains(&import_line) {
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

    fn as_cleanable(&self) -> Option<&dyn Cleanable> {
        Some(self)
    }
}

impl Cleanable for KittyGenerator {
    fn cleanup(&self, paths: &IrisPaths) -> anyhow::Result<()> {
        crate::modules::cleanable::default_cleanup(self, paths)
    }

    fn remove_theme(&self, paths: &IrisPaths, theme_name: &str) -> anyhow::Result<()> {
        crate::modules::cleanable::default_remove(self, paths, theme_name)
    }
}

/// Tests for kitty generator
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{core::IrisContext, models::Theme};

    /// Unit-tests for kitty
    mod unit {
        use super::*;

        #[test]
        fn should_return_metadata_for_kitty() {
            let generator = KittyGenerator;
            assert_eq!(generator.name(), "kitty");
            assert_eq!(generator.generator_type(), GeneratorType::Terminal);
            assert_eq!(generator.target_file_name(""), "current_theme.conf");
        }

        #[test]
        fn should_build_valid_render_context_for_kitty() {
            let generator = KittyGenerator;
            let (_, ctx) = IrisContext::mock();
            let theme: Theme = Theme::mock();
            let ctx = ctx.engine(&theme).build_context(&generator).unwrap();

            assert_eq!(ctx.get("bg").unwrap().as_str().unwrap(), theme.colors.bg);
            assert_eq!(ctx.get("sel").unwrap().as_str().unwrap(), theme.colors.sel);
            assert!(ctx.get("ansi").unwrap().is_array());
        }

        #[test]
        fn should_apply_theme_for_kitty() {
            let (_, ctx) =
                IrisContext::with_templates(vec![("terminals/kitty", "background {{ bg }}")]);
            let generator = KittyGenerator;
            let theme: Theme = Theme::mock();

            let kitty_dir = generator.resolve_config_directory(&ctx.paths);
            fs::create_dir_all(&kitty_dir).unwrap();

            let mut activity = ctx.log.step("Test", false).muted();
            let result = ctx.engine(&theme).execute_apply(&generator, &mut activity);
            assert!(result.is_ok(), "Failed to apply: {:?}", result.err());

            let cache_file = ctx.paths.generators.join("kitty").join("test-theme.conf");
            assert!(cache_file.exists());

            let content = fs::read_to_string(cache_file).unwrap();
            assert!(content.contains("background"));
        }

        #[test]
        fn should_clear_generated_files_for_kitty() {
            let (_, ctx) = IrisContext::mock();
            let generator = KittyGenerator;

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
        fn should_remove_theme_for_kitty() {
            let (_, ctx) = IrisContext::mock();
            let generator = KittyGenerator;

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

    /// Integration tests for kitty
    mod integration {
        use super::*;
        use crate::skip_if_not_installed;

        #[test]
        fn should_return_health_ok_for_kitty() {
            skip_if_not_installed!(KittyGenerator);

            let (_, mut ctx) = IrisContext::mock();
            let generator = KittyGenerator;
            let theme: Theme = Theme::mock();

            let mut activity = ctx.log.step("Test", false).muted();
            ctx.state.theme.current_theme = theme.name.clone();
            ctx.engine(&theme)
                .execute_apply(&generator, &mut activity)
                .unwrap();

            let config_path = generator
                .resolve_config_directory(&ctx.paths)
                .join("kitty.conf");

            let content = format!("include {}", generator.target_file_name(""));
            fs::write(&config_path, content).unwrap();

            let status = generator.health_check(&ctx.paths, &theme.name);
            assert!(status.is_ok(), "Expected Ok, got: {status}");
        }

        #[test]
        fn should_return_health_error_missing_config_for_kitty() {
            skip_if_not_installed!(KittyGenerator);

            let (_, ctx) = IrisContext::mock();
            let generator = KittyGenerator;
            let theme: Theme = Theme::mock();

            let status = generator.health_check(&ctx.paths, &theme.name);
            assert!(status.is_error(), "Expected Error, got: {status}");
            assert!(status.contains("Configuration file missing"));
        }

        #[test]
        fn should_return_health_warning_no_import_for_kitty() {
            skip_if_not_installed!(KittyGenerator);

            let (_, mut ctx) = IrisContext::mock();
            let generator = KittyGenerator;
            let theme: Theme = Theme::mock();

            let mut activity = ctx.log.step("Test", false).muted();
            ctx.state.theme.current_theme = theme.name.clone();
            ctx.engine(&theme)
                .execute_apply(&generator, &mut activity)
                .unwrap();

            let config_path = generator
                .resolve_config_directory(&ctx.paths)
                .join("kitty.conf");
            fs::create_dir_all(config_path.parent().unwrap()).unwrap();
            fs::write(&config_path, "font_size 18").unwrap();

            let status = generator.health_check(&ctx.paths, &theme.name);

            assert!(status.is_warning(), "Expected Warning, got: {status}");
            assert!(status.contains("Theme not imported"));
        }

        #[test]
        fn should_fix_inject_issue_for_kitty() {
            skip_if_not_installed!(KittyGenerator);

            let (_, ctx) = IrisContext::mock();
            let generator = KittyGenerator;
            let theme: Theme = Theme::mock();

            let config_dir = generator.resolve_config_directory(&ctx.paths);
            fs::create_dir_all(&config_dir).unwrap();
            let config_path = config_dir.join("kitty.conf");

            let mut activity = ctx.log.step("Test", false).muted();
            let engine = ctx.engine(&theme);
            engine.execute_apply(&generator, &mut activity).unwrap();
            fs::write(&config_path, "font_size 12").unwrap();

            let status = generator.health_check(&ctx.paths, &theme.name);
            generator.fix(&status, &engine, &mut activity).unwrap();

            let content = fs::read_to_string(&config_path).unwrap();
            assert!(content.contains("current_theme.conf"));
            assert!(generator.health_check(&ctx.paths, &theme.name).is_ok());
        }

        #[test]
        fn should_fix_broken_link_for_kitty() {
            skip_if_not_installed!(KittyGenerator);

            let (_, mut ctx) = IrisContext::mock();
            let generator = KittyGenerator;
            let theme: Theme = Theme::mock();
            ctx.state.theme.current_theme = theme.name.clone();

            let mut activity = ctx.log.step("Test", false).muted();
            let kitty_dir = generator.resolve_config_directory(&ctx.paths);
            let config_path = kitty_dir.join("kitty.conf");
            fs::create_dir_all(&kitty_dir).unwrap();

            let content = format!("include {}", generator.target_file_name(""));
            fs::write(&config_path, content).unwrap();

            let engine = ctx.engine(&theme);
            engine.execute_apply(&generator, &mut activity).unwrap();

            let link_path_empty = generator.link_path(&ctx.paths, "");
            if link_path_empty.exists() {
                fs::remove_file(&link_path_empty).unwrap();
            }

            let status = generator.health_check(&ctx.paths, &theme.name);
            assert!(status.is_error());
            assert!(status.contains("Invalid symlink"));

            generator.fix(&status, &engine, &mut activity).unwrap();
            assert!(generator.health_check(&ctx.paths, &theme.name).is_ok());
        }
    }
}
