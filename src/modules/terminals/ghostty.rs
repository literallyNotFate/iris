use crate::{
    core::{InjectionPosition, IrisEngine},
    infra::IrisPaths,
    models::{HealthStatus, Issue},
    modules::{Generator, GeneratorType, Strategy, traits::*},
};
use std::{fs, path::PathBuf};

/// Config generator for ghostty terminal
pub struct GhosttyGenerator;

impl Identifiable for GhosttyGenerator {
    fn name(&self) -> &'static str {
        "ghostty"
    }

    fn generator_type(&self) -> GeneratorType {
        GeneratorType::Terminal
    }
}

impl PathResolvable for GhosttyGenerator {
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
}

impl Generator for GhosttyGenerator {
    fn strategy(&self) -> Strategy {
        Strategy::Symlink
    }

    fn pre_apply(&self, engine: &IrisEngine) -> anyhow::Result<()> {
        let config_path: PathBuf = self.resolve_config_directory(engine.paths).join("config");
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        engine.inject_line(
            &config_path,
            &format!("config-file = {}", self.target_file_name("")),
            InjectionPosition::Start,
        )
    }
}

impl Diagnosable for GhosttyGenerator {
    fn health_check(&self, paths: &IrisPaths, theme: &str) -> HealthStatus {
        if !self.is_installed() {
            return HealthStatus::error(Issue::BinaryNotFound);
        }

        let ghostty_dir: PathBuf = self.resolve_config_directory(paths);
        let config_path: PathBuf = ghostty_dir.join("config");
        let link_path: PathBuf = self.link_path(paths, "");

        let config_status = HealthStatus::check_file(&config_path, Issue::ConfigMissing);
        if !config_status.is_ok() {
            return config_status;
        }

        let content: String = fs::read_to_string(&config_path).unwrap_or_default();
        let import_line: String = format!("config-file = {}", self.target_file_name(""));
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

impl Cleanable for GhosttyGenerator {
    fn cleanup(&self, paths: &IrisPaths) -> anyhow::Result<()> {
        default_cleanup(self, paths)
    }

    fn remove_theme(&self, paths: &IrisPaths, theme_name: &str) -> anyhow::Result<()> {
        default_remove(self, paths, theme_name)
    }
}

impl Diffable for GhosttyGenerator {
    fn config_path(&self, paths: &IrisPaths) -> PathBuf {
        self.resolve_config_directory(paths).join("config")
    }

    fn diff_style(&self) -> DiffStyle {
        DiffStyle::InjectKey {
            key_prefix: "config-file".to_string(),
            build_ideal_line: |_| "config-file = current_theme.conf".to_string(),
            at_top: true,
        }
    }
}

/// Tests for ghostty generator
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{core::IrisContext, models::Theme};

    /// Unit-tests for ghostty
    mod unit {
        use super::*;

        #[test]
        fn should_return_metadata_for_ghostty() {
            let generator = GhosttyGenerator;
            assert_eq!(generator.name(), "ghostty");
            assert_eq!(generator.generator_type(), GeneratorType::Terminal);
            assert_eq!(generator.target_file_name("melange"), "melange.conf");
        }

        #[test]
        fn should_build_valid_render_context_for_ghostty() {
            let generator = GhosttyGenerator;
            let (_, ctx) = IrisContext::mock();
            let theme: Theme = Theme::mock();
            let ctx = ctx.engine(&theme).build_context(&generator).unwrap();

            assert_eq!(ctx.get("bg").unwrap().as_str().unwrap(), theme.colors.bg);
            assert!(ctx.get("ansi").unwrap().is_array());
        }

        #[test]
        fn should_apply_theme_for_ghostty() {
            let (_, ctx) =
                IrisContext::with_templates(vec![("terminals/ghostty", "background = {{ bg }}")]);
            let generator = GhosttyGenerator;
            let theme: Theme = Theme::mock();

            let ghostty_dir = generator.resolve_config_directory(&ctx.paths);
            fs::create_dir_all(&ghostty_dir).unwrap();

            let mut activity = ctx.log.step("Test", false).muted();
            let result = ctx.engine(&theme).execute_apply(&generator, &mut activity);
            assert!(result.is_ok(), "Failed to apply: {:?}", result.err());

            let cache_file = ctx.paths.generators.join("ghostty").join("test-theme.conf");
            assert!(cache_file.exists());

            let content = fs::read_to_string(cache_file).unwrap();
            assert!(content.contains("background ="));
        }

        #[test]
        fn should_cleanup_generated_files_for_ghostty() {
            let (_, ctx) = IrisContext::mock();
            let generator = GhosttyGenerator;

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
        fn should_remove_theme_for_ghostty() {
            let (_, ctx) = IrisContext::mock();
            let generator = GhosttyGenerator;

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

    /// Integration tests for ghostty
    mod integration {
        use super::*;
        use crate::skip_if_not_installed;

        #[test]
        fn should_return_health_ok_for_ghostty() {
            skip_if_not_installed!(GhosttyGenerator);

            let (_, mut ctx) = IrisContext::mock();
            let generator = GhosttyGenerator;
            let theme: Theme = Theme::mock();
            ctx.state.theme.current_theme = theme.name.clone();

            let ghostty_dir = generator.resolve_config_directory(&ctx.paths);
            fs::create_dir_all(&ghostty_dir).unwrap();
            let mut activity = ctx.log.step("Test", false).muted();
            ctx.engine(&theme)
                .execute_apply(&generator, &mut activity)
                .unwrap();

            let config_path = generator
                .resolve_config_directory(&ctx.paths)
                .join("config");
            let import_line = format!("config-file = {}", generator.target_file_name(""));
            fs::write(&config_path, import_line).unwrap();

            let status = generator.health_check(&ctx.paths, &theme.name);
            assert!(status.is_ok(), "Expected Ok, got: {status}");
        }

        #[test]
        fn should_return_health_error_missing_config_for_ghostty() {
            skip_if_not_installed!(GhosttyGenerator);

            let (_, ctx) = IrisContext::mock();
            let generator = GhosttyGenerator;
            let theme: Theme = Theme::mock();
            let ghostty_dir = generator.resolve_config_directory(&ctx.paths);
            fs::create_dir_all(&ghostty_dir).unwrap();

            let status = generator.health_check(&ctx.paths, &theme.name);
            assert!(status.is_error(), "Expected Error, got: {status}");
            assert!(status.contains("configuration file missing"));
        }

        #[test]
        fn should_return_health_warning_no_import_for_ghostty() {
            skip_if_not_installed!(GhosttyGenerator);

            let (_, mut ctx) = IrisContext::mock();
            let generator = GhosttyGenerator;
            let theme: Theme = Theme::mock();
            ctx.state.theme.current_theme = theme.name.clone();
            let ghostty_dir = generator.resolve_config_directory(&ctx.paths);
            fs::create_dir_all(&ghostty_dir).unwrap();

            let mut activity = ctx.log.step("Test", false).muted();
            ctx.engine(&theme)
                .execute_apply(&generator, &mut activity)
                .unwrap();

            let config_path = generator
                .resolve_config_directory(&ctx.paths)
                .join("config");
            fs::write(&config_path, "font-family = JetBrainsMono").unwrap();

            let status = generator.health_check(&ctx.paths, &theme.name);

            assert!(status.is_warning(), "Expected warning, got: {status}");
            assert!(status.contains("not imported"));
        }

        #[test]
        fn should_fix_inject_issue_for_ghostty() {
            skip_if_not_installed!(GhosttyGenerator);

            let (_, ctx) = IrisContext::mock();
            let generator = GhosttyGenerator;
            let theme: Theme = Theme::mock();
            let engine = ctx.engine(&theme);

            let config_dir = generator.resolve_config_directory(&ctx.paths);
            fs::create_dir_all(&config_dir).unwrap();
            let config_path = config_dir.join("config");

            let mut activity = ctx.log.step("Test", false).muted();
            engine.execute_apply(&generator, &mut activity).unwrap();
            fs::write(&config_path, "font-size = 12").unwrap();

            let status = generator.health_check(&ctx.paths, &theme.name);
            engine
                .execute_fix(&generator, &status, &mut activity)
                .unwrap();

            let content = fs::read_to_string(&config_path).unwrap();
            assert!(content.contains("current_theme.conf"));
            assert!(generator.health_check(&ctx.paths, &theme.name).is_ok());
        }

        #[test]
        fn should_fix_broken_link_for_ghostty() {
            skip_if_not_installed!(GhosttyGenerator);

            let (_, mut ctx) = IrisContext::mock();
            let generator = GhosttyGenerator;
            let theme: Theme = Theme::mock();
            ctx.state.theme.current_theme = theme.name.clone();

            let engine = ctx.engine(&theme);
            let mut activity = ctx.log.step("Test", false).muted();
            let ghostty_dir = generator.resolve_config_directory(&ctx.paths);
            let config_path = ghostty_dir.join("config");
            fs::create_dir_all(&ghostty_dir).unwrap();

            let import_line = format!("config-file = {}", generator.target_file_name(""));
            fs::write(&config_path, import_line).unwrap();

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
            assert!(status.contains("invalid symlink"));

            engine
                .execute_fix(&generator, &status, &mut activity)
                .unwrap();
            assert!(generator.health_check(&ctx.paths, &theme.name).is_ok());
        }
    }
}
