use crate::{
    core::{InjectionPosition, IrisEngine},
    infra::IrisPaths,
    models::{HealthStatus, Issue},
    modules::{Generator, GeneratorType, Strategy, traits::*},
};
use std::{fs, path::PathBuf};

/// Config generator for btop utility
pub struct BtopGenerator;

impl Identifiable for BtopGenerator {
    fn name(&self) -> &'static str {
        "btop"
    }

    fn generator_type(&self) -> GeneratorType {
        GeneratorType::System
    }
}

impl PathResolvable for BtopGenerator {
    fn base_file_name(&self) -> String {
        "btop.conf".into()
    }

    fn file_name(&self, theme: &str) -> String {
        format!("{}.theme", theme.to_lowercase())
    }

    fn link_path(&self, paths: &IrisPaths, theme: &str) -> PathBuf {
        self.config_dir(paths)
            .join("themes")
            .join(self.file_name(theme))
    }
}

impl Generator for BtopGenerator {
    fn strategy(&self) -> Strategy {
        Strategy::Symlink
    }

    fn pre_apply(&self, engine: &IrisEngine) -> anyhow::Result<()> {
        let config_path: PathBuf = self.config_path(engine.paths);
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let link_path: PathBuf = self.link_path(engine.paths, &engine.theme.name);
        if let Some(parent) = link_path.parent() {
            fs::create_dir_all(parent)?;
        }

        engine.remove_marker(&config_path, "color_theme")?;
        engine.inject_line(
            &config_path,
            &format!("color_theme = \"{}\"", engine.theme.name.to_lowercase()),
            InjectionPosition::Start,
        )
    }
}

impl Diagnosable for BtopGenerator {
    fn health_check(&self, paths: &IrisPaths, theme: &str) -> HealthStatus {
        if !self.is_installed() {
            return HealthStatus::error(Issue::BinaryNotFound);
        }

        let conf_path: PathBuf = self.config_path(paths);
        let link_path: PathBuf = self.link_path(paths, theme);

        let config_status = HealthStatus::check_file(&conf_path, Issue::ConfigMissing);
        if !config_status.is_ok() {
            return config_status;
        }

        if !theme.is_empty() {
            let content: String = fs::read_to_string(&conf_path).unwrap_or_default();
            let expected_line: String = format!("color_theme = \"{}\"", theme.to_lowercase());

            if !content.contains(&expected_line) {
                return HealthStatus::warn(Issue::MarkerMissing);
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
        }

        HealthStatus::Ok
    }
}

impl Cleanable for BtopGenerator {
    fn cleanup(&self, paths: &IrisPaths) -> anyhow::Result<()> {
        default_cleanup(self, paths)
    }

    fn remove_theme(&self, paths: &IrisPaths, theme_name: &str) -> anyhow::Result<()> {
        default_remove(self, paths, theme_name)
    }
}

impl Diffable for BtopGenerator {
    fn diff_style(&self) -> DiffStyle {
        DiffStyle::InjectKey {
            key_prefix: "color_theme".to_string(),
            build_ideal_line: |theme| {
                let theme_name = if theme.is_empty() { "default" } else { theme };
                format!("color_theme = \"{}\"", theme_name.to_lowercase())
            },
            at_top: true,
        }
    }
}

/// Tests for btop generator
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{core::IrisContext, models::Theme};

    /// Unit-tests for btop
    mod unit {
        use super::*;

        #[test]
        fn should_return_metadata_for_btop() {
            let generator = BtopGenerator;
            assert_eq!(generator.name(), "btop");
            assert_eq!(generator.generator_type(), GeneratorType::System);
            assert_eq!(generator.file_name("iris-dark"), "iris-dark.theme");
        }

        #[test]
        fn should_handle_path_resolution_for_btop() {
            let (_temp_dir, ctx) = IrisContext::mock();
            let generator = BtopGenerator;
            let theme = "tokyonight";

            let expected_config_dir = ctx.paths.config.parent().unwrap().join("btop");
            assert_eq!(generator.config_dir(&ctx.paths), expected_config_dir);

            let expected_config_path = expected_config_dir.join("btop.conf");
            assert_eq!(generator.config_path(&ctx.paths), expected_config_path);

            let expected_cache_path = ctx.paths.generators.join("btop/tokyonight.theme");
            assert_eq!(generator.cache_path(&ctx.paths, theme), expected_cache_path);

            let expected_link_path = expected_config_dir.join("themes/tokyonight.theme");
            assert_eq!(generator.link_path(&ctx.paths, theme), expected_link_path);

            assert_eq!(generator.template_path(), "system/btop");
        }

        #[test]
        fn should_build_valid_render_context_for_btop() {
            let generator = BtopGenerator;
            let (_, ctx) = IrisContext::mock();
            let theme: Theme = Theme::mock();
            let ctx = ctx.engine(&theme).build_context(&generator).unwrap();

            assert_eq!(ctx.get("bg").unwrap().as_str().unwrap(), theme.colors.bg);
            assert_eq!(ctx.get("fg").unwrap().as_str().unwrap(), theme.colors.fg);
            assert!(ctx.contains_key("type_name"));
            assert!(ctx.contains_key("theme_name"));
        }

        #[test]
        fn should_apply_theme_for_btop() {
            let (_, ctx) =
                IrisContext::with_templates(vec![("system/btop", "theme[main_bg]=\"{{ bg }}]\"")]);
            let generator = BtopGenerator;
            let theme: Theme = Theme::mock();

            let btop_conf = generator.config_path(&ctx.paths);
            let btop_root = btop_conf.parent().unwrap();

            fs::create_dir_all(btop_root).unwrap();
            fs::write(
                &btop_conf,
                "graph_symbol = \"braille\"\ncolor_theme = \"old-theme\"\n",
            )
            .unwrap();

            let mut activity = ctx.log.step("Test", false).muted();
            ctx.engine(&theme)
                .execute_apply(&generator, &mut activity)
                .unwrap();

            let updated_content = fs::read_to_string(&btop_conf).expect("Could not read btop.conf");
            let expected_line = format!("color_theme = \"{}\"", theme.name.to_lowercase());

            assert!(updated_content.contains(&expected_line));
            assert!(
                updated_content.contains("graph_symbol = \"braille\""),
                "File content corrupted"
            );
        }

        #[test]
        fn should_clear_generated_files_for_btop() {
            let (_, ctx) = IrisContext::mock();
            let generator = BtopGenerator;

            let cache_dir = ctx.paths.generators.join(generator.name());
            fs::create_dir_all(&cache_dir).unwrap();

            let test_file = cache_dir.join("test.theme");
            fs::write(&test_file, "theme content").unwrap();

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
        fn should_remove_theme_for_btop() {
            let (_, ctx) = IrisContext::mock();
            let generator = BtopGenerator;

            let cache_file = generator.cache_path(&ctx.paths, "test");
            let link_file = generator.link_path(&ctx.paths, "test");

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

    /// Integration tests for btop
    mod integration {
        use super::*;
        use crate::skip_if_not_installed;

        #[test]
        fn should_return_health_ok_for_btop() {
            skip_if_not_installed!(BtopGenerator);

            let (_, mut ctx) = IrisContext::mock();
            let generator = BtopGenerator;
            let theme: Theme = Theme::mock();

            let conf_path = generator.config_path(&ctx.paths);
            fs::create_dir_all(conf_path.parent().unwrap()).unwrap();

            let mut activity = ctx.log.step("Test", false).muted();
            ctx.state.theme.current_theme = theme.name.clone();
            ctx.engine(&theme)
                .execute_apply(&generator, &mut activity)
                .unwrap();

            let expected_line = format!("color_theme = \"{}\"", theme.name);
            fs::write(
                &conf_path,
                format!("graph_symbol = \"braille\"\n{}", expected_line),
            )
            .unwrap();

            let status = generator.health_check(&ctx.paths, &theme.name);
            assert!(status.is_ok(), "Expected Ok, got: {status}");
        }

        #[test]
        fn should_return_health_error_missing_conf_for_btop() {
            skip_if_not_installed!(BtopGenerator);

            let (_, ctx) = IrisContext::mock();
            let generator = BtopGenerator;
            let status = generator.health_check(&ctx.paths, &ctx.state.theme.current_theme);

            assert!(status.is_error(), "Expected Error, got: {status}");
            assert!(status.contains("Configuration file missing"));
        }

        #[test]
        fn should_return_health_warning_wrong_theme_in_conf_for_btop() {
            skip_if_not_installed!(BtopGenerator);

            let (_, ctx) = IrisContext::mock();
            let generator = BtopGenerator;
            let theme: Theme = Theme::mock();

            let btop_conf = generator.config_path(&ctx.paths);
            let btop_root = btop_conf.parent().unwrap();

            fs::create_dir_all(btop_root).unwrap();
            fs::write(&btop_conf, "color_theme = \"default\"\n").unwrap();
            let status = generator.health_check(&ctx.paths, &theme.name);

            assert!(status.is_warning(), "Expected Warning, got: {status}");
            assert!(
                status.contains("Marker missing"),
                "Expected Warning, got: {status}"
            );
        }

        #[test]
        fn should_fix_broken_conf_for_btop() {
            skip_if_not_installed!(BtopGenerator);

            let (_, ctx) = IrisContext::mock();
            let generator = BtopGenerator;
            let theme: Theme = Theme::mock();

            let conf_path = generator.config_path(&ctx.paths);
            let btop_dir = conf_path.parent().unwrap();
            fs::create_dir_all(btop_dir).unwrap();
            fs::write(
                &conf_path,
                "color_theme = \"wrong_theme\"\nother_setting = true",
            )
            .unwrap();

            let status = generator.health_check(&ctx.paths, &theme.name);

            assert!(status.is_warning(), "Expected Warning, got: {status}");
            assert!(status.contains("Marker missing"));

            let mut activity = ctx.log.step("Test", false).muted();
            let engine = ctx.engine(&theme);
            engine
                .execute_fix(&generator, &status, &mut activity)
                .unwrap();

            let content = fs::read_to_string(&conf_path).unwrap();
            assert!(content.contains(&format!("color_theme = \"{}\"", theme.name)));
            assert!(content.contains("other_setting = true"));
        }

        #[test]
        fn should_fix_missing_theme_file_for_btop() {
            skip_if_not_installed!(BtopGenerator);

            let (_, mut ctx) = IrisContext::mock();
            let generator = BtopGenerator;
            let theme: Theme = Theme::mock();

            ctx.state.theme.current_theme = theme.name.clone();
            let conf_path = generator.config_path(&ctx.paths);
            let btop_dir = conf_path.parent().unwrap();
            fs::create_dir_all(btop_dir.join("themes")).unwrap();

            fs::write(&conf_path, format!("color_theme = \"{}\"", theme.name)).unwrap();

            let mut activity = ctx.log.step("Test", false).muted();
            let engine = ctx.engine(&theme);
            engine.execute_apply(&generator, &mut activity).unwrap();

            let link_path = generator.link_path(&ctx.paths, &theme.name);
            assert!(link_path.exists());

            fs::remove_file(&link_path).unwrap();

            let status = generator.health_check(&ctx.paths, &theme.name);
            assert!(status.is_error(), "Expected Error, got: {status}");

            engine
                .execute_fix(&generator, &status, &mut activity)
                .unwrap();

            assert!(link_path.exists(), "Fix should restore the symlink");
            assert!(generator.health_check(&ctx.paths, &theme.name).is_ok());
        }
    }
}
