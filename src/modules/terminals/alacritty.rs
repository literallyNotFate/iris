use crate::{
    core::{InjectionPosition, IrisEngine},
    infra::IrisPaths,
    models::{HealthStatus, Issue},
    modules::{Generator, GeneratorType, Strategy, traits::*},
};
use std::{fs, path::PathBuf};

/// Config generator for Alacritty terminal
pub struct AlacrittyGenerator;

impl Identifiable for AlacrittyGenerator {
    fn name(&self) -> &'static str {
        "alacritty"
    }

    fn generator_type(&self) -> GeneratorType {
        GeneratorType::Terminal
    }
}

impl PathResolvable for AlacrittyGenerator {
    fn target_file_name(&self, theme: &str) -> String {
        if theme.is_empty() {
            "current_theme.toml".into()
        } else {
            format!("{}.toml", theme.to_lowercase())
        }
    }

    fn link_path(&self, paths: &IrisPaths, _theme: &str) -> PathBuf {
        self.resolve_config_directory(paths)
            .join(self.target_file_name(""))
    }

    fn active_link_path(&self, paths: &IrisPaths) -> Option<PathBuf> {
        Some(
            self.resolve_config_directory(paths)
                .join("current_theme.toml"),
        )
    }
}

impl Generator for AlacrittyGenerator {
    fn strategy(&self) -> Strategy {
        Strategy::Symlink
    }

    fn pre_apply(&self, engine: &IrisEngine) -> anyhow::Result<()> {
        let config_path: PathBuf = self
            .resolve_config_directory(engine.paths)
            .join("alacritty.toml");
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        engine.inject_line(
            &config_path,
            "import = [\"~/.config/alacritty/current_theme.toml\"]",
            InjectionPosition::After("[general]".to_string()),
        )
    }
}

impl Diagnosable for AlacrittyGenerator {
    fn health_check(&self, paths: &IrisPaths, theme: &str) -> HealthStatus {
        if !self.is_installed() {
            return HealthStatus::error(Issue::BinaryNotFound);
        }

        let alacritty_dir: PathBuf = self.resolve_config_directory(paths);
        let config_path: PathBuf = alacritty_dir.join("alacritty.toml");
        let link_path: PathBuf = self.link_path(paths, "");

        let config_status = HealthStatus::check_file(&config_path, Issue::ConfigMissing);
        if !config_status.is_ok() {
            return config_status;
        }

        let content: String = fs::read_to_string(&config_path).unwrap_or_default();
        let import_line: &str = "import = [\"~/.config/alacritty/current_theme.toml\"]";
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

impl Cleanable for AlacrittyGenerator {
    fn cleanup(&self, paths: &IrisPaths) -> anyhow::Result<()> {
        default_cleanup(self, paths)
    }

    fn remove_theme(&self, paths: &IrisPaths, theme_name: &str) -> anyhow::Result<()> {
        default_remove(self, paths, theme_name)
    }
}

impl Diffable for AlacrittyGenerator {
    fn config_path(&self, paths: &IrisPaths) -> PathBuf {
        self.resolve_config_directory(paths).join("alacritty.toml")
    }

    fn diff_style(&self) -> DiffStyle {
        DiffStyle::Custom(Box::new(|current_content, _, config_path, _| {
            let expected = "~/.config/alacritty/current_theme.toml";
            let mut doc = current_content
                .parse::<toml_edit::DocumentMut>()
                .map_err(|_| anyhow::anyhow!("Failed to parse alacritty.toml as TOML"))?;

            if !doc.contains_key("general") {
                doc["general"] = toml_edit::table();
            }
            let general = doc["general"].as_table_mut().unwrap();

            let already_correct = general
                .get("import")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.get(0))
                .and_then(|val| val.as_str())
                .map_or(false, |s| s == expected);

            if already_correct {
                return Ok(None);
            }

            let mut import_array = toml_edit::Array::new();
            import_array.push(expected);
            general.insert("import", toml_edit::value(import_array));

            for (key, item) in doc.iter_mut() {
                if key != "general" {
                    if let Some(table) = item.as_table_mut() {
                        table.remove("import");
                    }
                }
            }

            let target_content = doc.to_string();
            if current_content.trim() == target_content.trim() {
                return Ok(None);
            }

            diffable::render_diff(config_path, current_content, &target_content)
        }))
    }
}

/// Tests for alacritty generator
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{core::IrisContext, models::Theme};

    /// Unit-tests for alacritty
    mod unit {
        use super::*;

        #[test]
        fn should_return_metadata_for_alacritty() {
            let generator = AlacrittyGenerator;
            assert_eq!(generator.name(), "alacritty");
            assert_eq!(generator.generator_type(), GeneratorType::Terminal);
            assert_eq!(generator.target_file_name("nord"), "nord.toml");
        }

        #[test]
        fn should_build_valid_render_context_for_alacritty() {
            let generator = AlacrittyGenerator;
            let (_, ctx) = IrisContext::mock();
            let theme: Theme = Theme::mock();
            let ctx = ctx.engine(&theme).build_context(&generator).unwrap();

            assert_eq!(ctx.get("bg").unwrap().as_str().unwrap(), theme.colors.bg);
            assert_eq!(ctx.get("fg").unwrap().as_str().unwrap(), theme.colors.fg);
            assert!(ctx.get("ansi").unwrap().is_array());
        }

        #[test]
        fn should_apply_theme_for_alacritty() {
            let (_, ctx) = IrisContext::with_templates(vec![(
                "terminals/alacritty",
                "[colors.primary]
                background = \"{{ bg }}\"",
            )]);
            let generator = AlacrittyGenerator;
            let theme: Theme = Theme::mock();

            let alacritty_dir = generator.resolve_config_directory(&ctx.paths);
            fs::create_dir_all(&alacritty_dir).unwrap();

            let mut activity = ctx.log.step("Test", false).muted();
            let result = ctx.engine(&theme).execute_apply(&generator, &mut activity);
            assert!(result.is_ok(), "Apply failed: {:?}", result.err());

            let cache_file = ctx
                .paths
                .generators
                .join("alacritty")
                .join("test-theme.toml");
            assert!(cache_file.exists(), "Theme missing in Iris cache");

            let alacritty_dir = generator.resolve_config_directory(&ctx.paths);
            let link_path = alacritty_dir.join("current_theme.toml");
            assert!(
                link_path.exists(),
                "Symlink missing in Alacritty config dir"
            );

            let content = fs::read_to_string(cache_file).unwrap();
            assert!(content.contains(&format!("background = \"{}\"", theme.colors.bg)));
        }

        #[test]
        fn should_clear_generated_files_for_alacritty() {
            let (_, ctx) = IrisContext::mock();
            let generator = AlacrittyGenerator;

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
        fn should_remove_theme_for_alacritty() {
            let (_, ctx) = IrisContext::mock();
            let generator = AlacrittyGenerator;

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

    /// Integration tests for alacritty
    mod integration {
        use super::*;
        use crate::skip_if_not_installed;

        #[test]
        fn should_return_health_ok_for_alacritty() {
            skip_if_not_installed!(AlacrittyGenerator);

            let (_, mut ctx) = IrisContext::mock();
            let generator = AlacrittyGenerator;
            let theme: Theme = Theme::mock();
            let alacritty_dir = generator.resolve_config_directory(&ctx.paths);
            fs::create_dir_all(&alacritty_dir).unwrap();

            let mut activity = ctx.log.step("Test", false).muted();
            ctx.state.theme.current_theme = theme.name.clone();
            ctx.engine(&theme)
                .execute_apply(&generator, &mut activity)
                .unwrap();

            let alacritty_dir = generator.resolve_config_directory(&ctx.paths);
            let main_config = alacritty_dir.join("alacritty.toml");
            fs::write(
                &main_config,
                "import = [\"~/.config/alacritty/current_theme.toml\"]",
            )
            .unwrap();

            let status = generator.health_check(&ctx.paths, &theme.name);
            assert!(status.is_ok(), "Expected Ok, got: {status}");
        }

        #[test]
        fn should_return_health_error_no_import_for_alacritty() {
            skip_if_not_installed!(AlacrittyGenerator);

            let (_, mut ctx) = IrisContext::mock();
            let generator = AlacrittyGenerator;
            let theme: Theme = Theme::mock();
            let alacritty_dir = generator.resolve_config_directory(&ctx.paths);
            fs::create_dir_all(&alacritty_dir).unwrap();

            let mut activity = ctx.log.step("Test", false).muted();
            ctx.state.theme.current_theme = theme.name.clone();
            ctx.engine(&theme)
                .execute_apply(&generator, &mut activity)
                .unwrap();

            let main_config = generator
                .resolve_config_directory(&ctx.paths)
                .join("alacritty.toml");
            fs::write(&main_config, "[window]\ndecorations = \"none\"").unwrap();

            let status = generator.health_check(&ctx.paths, &theme.name);

            assert!(status.is_warning(), "Expected Warning, got: {status}");
            assert!(status.contains("Theme not imported"));
        }

        #[test]
        fn should_return_health_warning_no_main_config_for_alacritty() {
            skip_if_not_installed!(AlacrittyGenerator);

            let (_, mut ctx) = IrisContext::mock();
            let generator = AlacrittyGenerator;
            let theme: Theme = Theme::mock();
            let alacritty_dir = generator.resolve_config_directory(&ctx.paths);
            fs::create_dir_all(&alacritty_dir).unwrap();

            let mut activity = ctx.log.step("Test", false).muted();
            ctx.state.theme.current_theme = theme.name.clone();
            ctx.engine(&theme)
                .execute_apply(&generator, &mut activity)
                .unwrap();

            let main_config = generator
                .resolve_config_directory(&ctx.paths)
                .join("alacritty.toml");
            if main_config.exists() {
                fs::remove_file(main_config).unwrap();
            }

            let status = generator.health_check(&ctx.paths, &theme.name);

            assert!(status.is_error(), "Expected Error, got: {status}");
            assert!(status.contains("Configuration file missing"));
        }

        #[test]
        fn should_fix_inject_issue_for_alacritty() {
            skip_if_not_installed!(AlacrittyGenerator);

            let (_, ctx) = IrisContext::mock();
            let generator = AlacrittyGenerator;
            let theme: Theme = Theme::mock();

            let alacritty_dir = generator.resolve_config_directory(&ctx.paths);
            fs::create_dir_all(&alacritty_dir).unwrap();
            let config_path = alacritty_dir.join("alacritty.toml");

            let mut activity = ctx.log.step("Test", false).muted();
            let engine = ctx.engine(&theme);
            engine.execute_apply(&generator, &mut activity).unwrap();
            fs::write(&config_path, "[window]\ndecorations = \"none\"").unwrap();

            let status = generator.health_check(&ctx.paths, &theme.name);

            assert!(status.is_warning(), "Expected Warning, but got: {status}");
            assert!(status.contains("Theme not imported"));

            engine
                .execute_fix(&generator, &status, &mut activity)
                .unwrap();

            let content = fs::read_to_string(&config_path).unwrap();
            assert!(content.contains("current_theme.toml"));
            assert!(generator.health_check(&ctx.paths, &theme.name).is_ok());
        }

        #[test]
        fn should_fix_broken_symlink_for_alacritty() {
            skip_if_not_installed!(AlacrittyGenerator);

            let (_, ctx) = IrisContext::mock();
            let generator = AlacrittyGenerator;
            let theme: Theme = Theme::mock();

            let alacritty_dir = generator.resolve_config_directory(&ctx.paths);
            fs::create_dir_all(&alacritty_dir).unwrap();

            let config_path = alacritty_dir.join("alacritty.toml");
            fs::write(
                &config_path,
                "import = [\"~/.config/alacritty/current_theme.toml\"]",
            )
            .unwrap();

            let mut activity = ctx.log.step("Test", false).muted();
            let engine = ctx.engine(&theme);
            engine.execute_apply(&generator, &mut activity).unwrap();

            let link_path = generator.link_path(&ctx.paths, "");
            if link_path.exists() || link_path.is_symlink() {
                fs::remove_file(&link_path).unwrap();
            }

            let status = generator.health_check(&ctx.paths, &theme.name);
            assert!(status.is_error(), "Should be Error, got: {status}");

            engine
                .execute_fix(&generator, &status, &mut activity)
                .unwrap();

            let final_status = generator.health_check(&ctx.paths, &theme.name);
            assert!(final_status.is_ok());
        }
    }
}
