use crate::{
    infra::IrisPaths,
    models::{HealthStatus, Issue, Theme},
    modules::{Generator, GeneratorType, Strategy, traits::*},
};
use std::{fs, path::PathBuf};

/// Config generator for yazi
pub struct YaziGenerator;

impl Identifiable for YaziGenerator {
    fn name(&self) -> &'static str {
        "yazi"
    }

    fn generator_type(&self) -> GeneratorType {
        GeneratorType::Tool
    }
}

impl PathResolvable for YaziGenerator {
    fn target_file_name(&self, theme: &str) -> String {
        if theme.is_empty() {
            "theme.toml".into()
        } else {
            format!("{}.toml", theme.to_lowercase())
        }
    }

    fn link_path(&self, paths: &IrisPaths, _theme: &str) -> PathBuf {
        self.resolve_config_directory(paths)
            .join(self.target_file_name(""))
    }

    fn active_link_path(&self, paths: &IrisPaths) -> Option<PathBuf> {
        Some(self.resolve_config_directory(paths).join("theme.toml"))
    }
}

impl Generator for YaziGenerator {
    fn strategy(&self) -> Strategy {
        Strategy::Symlink
    }

    fn enrich_context(&self, context: &mut tera::Context, theme: &Theme) -> anyhow::Result<()> {
        let line_hl = if theme.colors.line_hl == "#cccccc" {
            &theme.colors.sel
        } else {
            &theme.colors.line_hl
        };
        context.insert("line_hl", line_hl);

        Ok(())
    }
}

impl Diagnosable for YaziGenerator {
    fn health_check(&self, paths: &IrisPaths, theme: &str) -> HealthStatus {
        if !self.is_installed() {
            return HealthStatus::error(Issue::BinaryNotFound);
        }

        let link_path: PathBuf = self.link_path(paths, "");

        let link_status = HealthStatus::check_symlink(&link_path, Issue::SymlinkInvalid);
        if !link_status.is_ok() {
            return link_status;
        }

        let expected_cache: PathBuf = self.cache_path(paths, theme);
        if !expected_cache.exists() {
            return HealthStatus::warn(Issue::CacheMissing);
        }

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

impl Cleanable for YaziGenerator {
    fn cleanup(&self, paths: &IrisPaths) -> anyhow::Result<()> {
        crate::modules::traits::default_cleanup(self, paths)
    }

    fn remove_theme(&self, paths: &IrisPaths, theme_name: &str) -> anyhow::Result<()> {
        crate::modules::traits::default_remove(self, paths, theme_name)
    }
}

impl Diffable for YaziGenerator {
    fn config_path(&self, paths: &IrisPaths) -> PathBuf {
        self.resolve_config_directory(paths).join("yazi.toml")
    }

    fn diff(&self, _: &IrisPaths, _: &str) -> anyhow::Result<Option<String>> {
        Ok(None)
    }
}

/// Tests for yazi generator
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::IrisContext;

    /// Unit-tests for yazi
    mod unit {
        use super::*;

        #[test]
        fn should_return_metadata_for_yazi() {
            let generator = YaziGenerator;
            assert_eq!(generator.name(), "yazi");
            assert_eq!(generator.generator_type(), GeneratorType::Tool);
            assert_eq!(generator.target_file_name("cattpuccin"), "cattpuccin.toml");
        }

        #[test]
        fn should_build_valid_render_context_for_yazi() {
            let generator = YaziGenerator;
            let (_, ctx) = IrisContext::mock();
            let mut theme: Theme = Theme::mock();

            theme.colors.line_hl = "#123456".to_string();
            let tctx = ctx.engine(&theme).build_context(&generator).unwrap();
            assert_eq!(tctx.get("line_hl").unwrap().as_str().unwrap(), "#123456");

            theme.colors.line_hl = "#cccccc".to_string();
            theme.colors.sel = "#ff0000".to_string();
            let tctx = ctx.engine(&theme).build_context(&generator).unwrap();

            assert_eq!(tctx.get("line_hl").unwrap().as_str().unwrap(), "#ff0000");
            assert!(tctx.get("ansi").unwrap().is_array());
        }

        #[test]
        fn should_apply_theme_for_yazi() {
            let (_, ctx) = IrisContext::with_templates(vec![(
                "tools/yazi",
                "[manager]
            cwd = { fg = \"{{ ansi.9 }}\", bold = true }",
            )]);
            let generator = YaziGenerator;
            let theme: Theme = Theme::mock();

            let mut activity = ctx.log.step("Test", false);
            ctx.engine(&theme)
                .execute_apply(&generator, &mut activity)
                .unwrap();

            let expected_yazi_dir = generator.resolve_config_directory(&ctx.paths);
            let yazi_theme_link = expected_yazi_dir.join("theme.toml");
            assert!(yazi_theme_link.exists());

            let cache_content = fs::read_to_string(yazi_theme_link).unwrap();
            assert!(cache_content.contains("[manager]"));
        }

        #[test]
        fn should_clear_generated_files_for_yazi() {
            let (_, ctx) = IrisContext::with_templates(vec![(
                "tools/yazi",
                "[manager]
            cwd = { fg = \"{{ ansi.9 }}\", bold = true }",
            )]);
            let generator = YaziGenerator;
            let theme: Theme = Theme::mock();

            let mut activity = ctx.log.step("Test", false);
            ctx.engine(&theme)
                .execute_apply(&generator, &mut activity)
                .unwrap();

            let cache_dir = ctx.paths.generators.join(generator.name());
            assert!(cache_dir.exists());

            generator.cleanup(&ctx.paths).unwrap();
            assert!(
                !cache_dir.exists(),
                "Clear should remove the entire cache dir"
            );
        }

        #[test]
        fn should_remove_theme_for_yazi() {
            let (_, ctx) = IrisContext::mock();
            let generator = YaziGenerator;
            let theme: Theme = Theme::mock();

            let mut activity = ctx.log.step("Test", false);
            ctx.engine(&theme)
                .execute_apply(&generator, &mut activity)
                .unwrap();

            let cache_file = generator.cache_path(&ctx.paths, &theme.name);
            let link_file = generator.theme_path(&ctx.paths, &theme.name);

            assert!(cache_file.exists());
            assert!(link_file.exists());

            generator.remove_theme(&ctx.paths, &theme.name).unwrap();

            assert!(!cache_file.exists());
            assert!(!link_file.exists());
        }
    }

    /// Integration tests for yazi
    mod integration {
        use super::*;
        use crate::skip_if_not_installed;

        #[test]
        fn should_return_health_ok_for_yazi() {
            skip_if_not_installed!(YaziGenerator);

            let (_, mut ctx) = IrisContext::mock();
            let generator = YaziGenerator;
            let theme: Theme = Theme::mock();

            ctx.state.theme.current_theme = theme.name.clone();
            let mut activity = ctx.log.step("Test", false);
            ctx.engine(&theme)
                .execute_apply(&generator, &mut activity)
                .unwrap();

            let status = generator.health_check(&ctx.paths, &theme.name);
            assert!(status.is_ok(), "Expected Ok, got: {status}");
        }

        #[test]
        fn should_return_health_error_missing_link_for_yazi() {
            skip_if_not_installed!(YaziGenerator);

            let (_, ctx) = IrisContext::mock();
            let generator = YaziGenerator;
            let link = generator.link_path(&ctx.paths, "");

            if link.exists() || link.is_symlink() {
                let _ = fs::remove_file(&link);
            }

            let status = generator.health_check(&ctx.paths, &ctx.state.theme.current_theme);
            assert!(status.is_error(), "Expected Error, got: {status}");
            assert!(status.contains("Invalid symlink"));
        }

        #[test]
        fn should_return_health_error_cache_mismatch_for_yazi() {
            skip_if_not_installed!(YaziGenerator);

            let (_, mut ctx) = IrisContext::mock();
            let generator = YaziGenerator;
            let theme: Theme = Theme::mock();
            ctx.state.theme.current_theme = theme.name.clone();

            let mut activity = ctx.log.step("Test", false).muted();
            ctx.engine(&theme)
                .execute_apply(&generator, &mut activity)
                .unwrap();

            let link_path = generator.link_path(&ctx.paths, "");
            let fake_wrong_target = ctx.paths.cache.join("wrong_theme.toml");
            fs::write(&fake_wrong_target, "").unwrap();

            if link_path.is_symlink() || link_path.exists() {
                fs::remove_file(&link_path).unwrap();
            }
            std::os::unix::fs::symlink(&fake_wrong_target, &link_path).unwrap();

            let status = generator.health_check(&ctx.paths, &theme.name);
            assert!(status.is_warning(), "Expected Warning, got: {status}");
            assert!(status.contains("Cache mismatch"));
        }

        #[test]
        fn should_fix_broken_symlink_for_yazi() {
            skip_if_not_installed!(YaziGenerator);

            let (_, ctx) = IrisContext::mock();
            let generator = YaziGenerator;
            let theme: Theme = Theme::mock();

            let mut activity = ctx.log.step("Test", false);
            let engine = ctx.engine(&theme);
            engine.execute_apply(&generator, &mut activity).unwrap();

            let link_path = generator.link_path(&ctx.paths, &theme.name);
            let cache_file = generator.cache_path(&ctx.paths, &theme.name);

            fs::remove_file(&link_path).unwrap();

            let status = generator.health_check(&ctx.paths, &theme.name);
            assert!(status.is_error());
            assert!(status.contains("Invalid symlink"));

            engine
                .execute_fix(&generator, &status, &mut activity)
                .unwrap();
            assert!(link_path.exists(), "Fix should recreate the symlink");

            #[cfg(unix)]
            {
                let target = fs::read_link(&link_path).unwrap();
                assert_eq!(target, cache_file);
            }

            assert!(generator.health_check(&ctx.paths, &theme.name).is_ok());
        }

        #[test]
        fn should_fix_missing_cache_for_yazi() {
            skip_if_not_installed!(YaziGenerator);

            let (_, ctx) = IrisContext::mock();
            let generator = YaziGenerator;
            let theme: Theme = Theme::mock();
            let mut activity = ctx.log.step("Test", false).muted();
            let engine = ctx.engine(&theme);

            engine.execute_apply(&generator, &mut activity).unwrap();

            let cache_file = generator.cache_path(&ctx.paths, &theme.name);
            let link_file = generator.link_path(&ctx.paths, "");

            fs::remove_file(&cache_file).unwrap();
            if link_file.exists() || link_file.is_symlink() {
                fs::remove_file(&link_file).unwrap();
            }

            let status = generator.health_check(&ctx.paths, &theme.name);
            assert!(status.is_error() || status.is_warning());

            engine
                .execute_fix(&generator, &status, &mut activity)
                .unwrap();

            assert!(cache_file.exists());
            assert!(generator.health_check(&ctx.paths, &theme.name).is_ok());
        }
    }
}
