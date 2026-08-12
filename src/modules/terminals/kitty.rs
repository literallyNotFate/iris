use crate::{
    core::{InjectionPosition, IrisEngine},
    infra::IrisPaths,
    models::{HealthStatus, Issue},
    modules::{Generator, GeneratorType, Strategy, traits::*},
};
use std::{fs, path::PathBuf};

/// Config generator for kitty terminal
pub struct KittyGenerator;

impl Identifiable for KittyGenerator {
    fn name(&self) -> &'static str {
        "kitty"
    }

    fn generator_type(&self) -> GeneratorType {
        GeneratorType::Terminal
    }
}

impl PathResolvable for KittyGenerator {
    fn base_file_name(&self) -> String {
        "kitty.conf".into()
    }

    fn file_name(&self, theme: &str) -> String {
        if theme.is_empty() {
            "current_theme.conf".into()
        } else {
            format!("{}.conf", theme.to_lowercase())
        }
    }

    fn link_path(&self, paths: &IrisPaths, _theme: &str) -> PathBuf {
        self.config_dir(paths).join(self.file_name(""))
    }

    fn active_link_path(&self, paths: &IrisPaths) -> Option<PathBuf> {
        Some(self.config_dir(paths).join(self.file_name("")))
    }
}

impl Generator for KittyGenerator {
    fn strategy(&self) -> Strategy {
        Strategy::Symlink
    }

    fn pre_apply(&self, engine: &IrisEngine) -> anyhow::Result<()> {
        let config_path: PathBuf = self.config_path(engine.paths);
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        engine.inject_line(
            &config_path,
            &format!("include {}", self.file_name("")),
            InjectionPosition::Start,
        )
    }
}

impl Diagnosable for KittyGenerator {
    fn health_check(&self, paths: &IrisPaths, theme: &str) -> HealthStatus {
        if !self.is_installed() {
            return HealthStatus::error(Issue::BinaryNotFound);
        }

        let config_path: PathBuf = self.config_path(paths);
        let link_path: PathBuf = self.link_path(paths, "");

        let config_status = HealthStatus::check_file(&config_path, Issue::ConfigMissing);
        if !config_status.is_ok() {
            return config_status;
        }

        let content: String = fs::read_to_string(&config_path).unwrap_or_default();
        let import_line: String = format!("include {}", self.file_name(""));
        if !content.contains(&import_line) {
            return HealthStatus::warn(Issue::ImportMissing);
        }

        let link_status = HealthStatus::check_symlink(&link_path, Issue::SymlinkInvalid);
        if !link_status.is_ok() {
            return link_status;
        }

        let expected_cache: PathBuf = self.cache_path(paths, theme);
        if let Ok(target) = fs::read_link(&link_path) {
            let resolved_target: PathBuf = if target.is_relative() {
                link_path
                    .parent()
                    .map(|p| p.join(&target))
                    .unwrap_or(target)
            } else {
                target
            };

            if resolved_target != expected_cache {
                return HealthStatus::warn(Issue::CacheMismatch);
            }
        }

        HealthStatus::Ok
    }
}

impl Cleanable for KittyGenerator {
    fn cleanup(&self, paths: &IrisPaths) -> anyhow::Result<()> {
        default_cleanup(self, paths)
    }

    fn remove_theme(&self, paths: &IrisPaths, theme_name: &str) -> anyhow::Result<()> {
        default_remove(self, paths, theme_name)
    }
}

impl Diffable for KittyGenerator {
    fn diff_style(&self) -> DiffStyle {
        DiffStyle::InjectKey {
            key_prefix: "include".to_string(),
            build_ideal_line: |_| "include current_theme.conf".to_string(),
            at_top: true,
        }
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
            assert_eq!(generator.file_name(""), "current_theme.conf");
        }

        #[test]
        fn should_handle_path_resolution_for_kitty() {
            let (_temp_dir, ctx) = IrisContext::mock();
            let generator = KittyGenerator;
            let theme = "tokyonight";

            let expected_config_dir = ctx.paths.config.parent().unwrap().join("kitty");
            assert_eq!(generator.config_dir(&ctx.paths), expected_config_dir);

            let expected_config_path = expected_config_dir.join("kitty.conf");
            assert_eq!(generator.config_path(&ctx.paths), expected_config_path);

            let expected_cache_path = ctx.paths.generators.join("kitty/tokyonight.conf");
            assert_eq!(generator.cache_path(&ctx.paths, theme), expected_cache_path);

            let expected_link_path = expected_config_dir.join("current_theme.conf");
            assert_eq!(generator.link_path(&ctx.paths, theme), expected_link_path);

            assert_eq!(
                generator.active_link_path(&ctx.paths),
                Some(expected_link_path)
            );
            assert_eq!(generator.template_path(), "terminals/kitty");
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

            let kitty_dir = generator.config_dir(&ctx.paths);
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
            let file = cache_dir.join(generator.file_name(""));
            fs::write(&file, "test").unwrap();

            assert!(cache_dir.exists());

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

            let config_path = generator.config_path(&ctx.paths);

            let content = format!("include {}", generator.file_name(""));
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

            let config_path = generator.config_path(&ctx.paths);
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

            let config_dir = generator.config_dir(&ctx.paths);
            fs::create_dir_all(&config_dir).unwrap();
            let config_path = config_dir.join("kitty.conf");

            let mut activity = ctx.log.step("Test", false).muted();
            let engine = ctx.engine(&theme);
            engine.execute_apply(&generator, &mut activity).unwrap();
            fs::write(&config_path, "font_size 12").unwrap();

            let status = generator.health_check(&ctx.paths, &theme.name);
            engine
                .execute_fix(&generator, &status, &mut activity)
                .unwrap();

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
            let kitty_dir = generator.config_dir(&ctx.paths);
            let config_path = kitty_dir.join("kitty.conf");
            fs::create_dir_all(&kitty_dir).unwrap();

            let content = format!("include {}", generator.file_name(""));
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

            engine
                .execute_fix(&generator, &status, &mut activity)
                .unwrap();
            assert!(generator.health_check(&ctx.paths, &theme.name).is_ok());
        }
    }
}
