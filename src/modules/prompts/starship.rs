use crate::{
    core::{InjectionPosition, IrisEngine},
    infra::IrisPaths,
    models::{HealthStatus, Issue},
    modules::{Generator, GeneratorType, Strategy, traits::*},
};
use std::{env, fs, path::PathBuf};

/// Config generator for starship
pub struct StarshipGenerator;

impl Identifiable for StarshipGenerator {
    fn name(&self) -> &str {
        "starship"
    }

    fn generator_type(&self) -> GeneratorType {
        GeneratorType::Prompt
    }
}

impl PathResolvable for StarshipGenerator {
    fn target_file_name(&self, _theme: &str) -> String {
        "starship.toml".into()
    }

    fn cache_path(&self, paths: &IrisPaths, theme: &str) -> PathBuf {
        paths
            .generators
            .join(self.name())
            .join(format!("{}_block.toml", theme))
    }

    fn link_path(&self, paths: &IrisPaths, _theme: &str) -> PathBuf {
        if let Some(env_path) = self.env_config_directory() {
            return env_path;
        }

        self.resolve_config_directory(paths)
            .join(self.target_file_name(""))
    }

    fn active_link_path(&self, paths: &IrisPaths) -> Option<PathBuf> {
        Some(self.resolve_config_directory(paths).join("starship.toml"))
    }

    fn env_config_directory(&self) -> Option<PathBuf> {
        env::var("STARSHIP_CONFIG").ok().map(PathBuf::from)
    }
}

impl Generator for StarshipGenerator {
    fn strategy(&self) -> Strategy {
        Strategy::InjectBlock {
            file: "starship.toml".to_string(),
        }
    }

    fn pre_apply(&self, engine: &IrisEngine) -> anyhow::Result<()> {
        let config_path: PathBuf = self.link_path(engine.paths, "");
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        engine.remove_key(&config_path, "palette")?;
        engine.inject_line(
            &config_path,
            &format!("palette = \"{}\"", engine.theme.name.to_lowercase()),
            InjectionPosition::Start,
        )?;

        Ok(())
    }
}

impl StarshipGenerator {
    pub fn remove_palette_block(&self, target_path: &PathBuf) -> anyhow::Result<()> {
        if !target_path.exists() {
            return Ok(());
        }

        let content: String = fs::read_to_string(target_path)?;
        let cleaned: String = crate::utils::remove_key(&content, "palette");
        let cleaned: String = crate::utils::replace_block(&cleaned, self.name(), "");

        fs::write(target_path, cleaned.trim())?;
        Ok(())
    }
}

impl Diagnosable for StarshipGenerator {
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
            let theme_lower: String = theme.to_lowercase();

            let expected_key: String = format!("palette = \"{}\"", theme_lower);
            if !content.contains(&expected_key) {
                return HealthStatus::warn(Issue::ImportMissing);
            }

            let palette_block: String = format!("[palettes.{}]", theme_lower);
            if !content.contains(&palette_block) {
                return HealthStatus::error(Issue::BlockMissing);
            }
        }

        HealthStatus::Ok
    }
}

impl Cleanable for StarshipGenerator {
    fn cleanup(&self, paths: &IrisPaths) -> anyhow::Result<()> {
        default_cleanup(self, paths)
    }

    fn remove_theme(&self, paths: &IrisPaths, theme_name: &str) -> anyhow::Result<()> {
        let config_path: PathBuf = self.link_path(paths, "");
        let theme_lower: String = theme_name.to_lowercase();

        if config_path.exists() {
            let content: String = fs::read_to_string(&config_path)?;
            let target_line = format!("palette = \"{}\"", theme_lower);
            if content.contains(&target_line) {
                self.remove_palette_block(&config_path)?;
            }
        }

        let cache_file: PathBuf = self.cache_path(paths, &theme_lower);
        if cache_file.exists() {
            fs::remove_file(cache_file)?;
        }

        Ok(())
    }

    fn cleanup_config(&self, config_path: &PathBuf) -> anyhow::Result<()> {
        self.remove_palette_block(config_path)
    }
}

/// Tests for starship generator
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{core::IrisContext, models::Theme};
    use tempdir::TempDir;

    /// Unit-tests for starship
    mod unit {
        use super::*;

        #[test]
        fn should_return_metadata_for_starship() {
            let generator = StarshipGenerator;
            assert_eq!(generator.name(), "starship");
            assert_eq!(generator.generator_type(), GeneratorType::Prompt);
            assert_eq!(generator.target_file_name("any"), "starship.toml");
        }

        #[test]
        fn should_build_valid_render_context_for_starship() {
            let generator = StarshipGenerator;
            let (_, ctx) = IrisContext::mock();
            let theme: Theme = Theme::mock();
            let ctx = ctx.engine(&theme).build_context(&generator).unwrap();

            assert_eq!(ctx.get("bg").unwrap().as_str().unwrap(), theme.colors.bg);
            assert_eq!(ctx.get("fg").unwrap().as_str().unwrap(), theme.colors.fg);
            assert!(ctx.get("ansi").unwrap().is_array());
        }

        #[test]
        fn should_clean_and_inject_correctly_for_starship() {
            let (_iris_dir, ctx) = IrisContext::with_templates(vec![(
                "prompts/starship",
                "[palettes.{{ theme_name }}]\nbase      = \"{{ bg }}\"",
            )]);
            let config_path = ctx.paths.config.join("starship.toml");
            let home_dir = ctx.paths.config.parent().unwrap();

            let initial_content = r##"
palette = "old_theme"
[directory]
style = "blue"

# [iris:begin:starship]
[palettes.old_theme]
base = "#000000"
# [iris:end:starship]
"##;
            fs::write(&config_path, initial_content).unwrap();

            temp_env::with_vars(
                [
                    ("STARSHIP_CONFIG", Some(config_path.as_path())),
                    ("HOME", Some(home_dir)),
                ],
                || {
                    let generator = StarshipGenerator;
                    let theme: Theme = Theme::mock();
                    let theme_lower = theme.name.to_lowercase();

                    let mut activity = ctx.log.step("Test", false).muted();
                    ctx.engine(&theme)
                        .execute_apply(&generator, &mut activity)
                        .unwrap();

                    let result = fs::read_to_string(&config_path).unwrap();
                    let palette_occurrences: Vec<_> = result.matches("palette =").collect();

                    assert_eq!(palette_occurrences.len(), 1);
                    assert!(result.contains(&format!("palette = \"{}\"", theme_lower)));
                    assert!(!result.contains("[palettes.old_theme]"));
                    assert!(result.contains(&format!("[palettes.{}]", theme_lower)));
                    assert!(result.contains("[directory]"));
                    assert!(result.contains("# [iris:begin:starship]"));
                    assert!(result.contains("# [iris:end:starship]"));
                },
            );
        }

        #[test]
        fn should_apply_theme_for_starship() {
            let (_iris_dir, ctx) = IrisContext::with_templates(vec![(
                "prompts/starship",
                "[palettes.{{ theme_name }}]\nbase      = \"{{ bg }}\"",
            )]);
            let config_path = ctx.paths.config.join("starship.toml");
            let home_dir = ctx.paths.config.parent().unwrap();
            fs::write(&config_path, "").unwrap();

            temp_env::with_vars(
                [
                    ("STARSHIP_CONFIG", Some(config_path.as_path())),
                    ("HOME", Some(home_dir)),
                ],
                || {
                    let generator = StarshipGenerator;
                    let theme: Theme = Theme::mock();
                    let theme_name_lower = theme.name.to_lowercase();

                    let mut activity = ctx.log.step("Test", false).muted();
                    let result = ctx.engine(&theme).execute_apply(&generator, &mut activity);
                    assert!(result.is_ok());

                    let final_content = fs::read_to_string(&config_path).unwrap();
                    assert!(final_content.contains(&format!("[palettes.{}]", theme_name_lower)));
                    assert!(final_content.contains("# [iris:begin:starship]"));
                    assert!(final_content.contains("# [iris:end:starship]"));
                },
            );
        }

        #[test]
        fn should_clear_generated_files_for_starship() {
            let base_tmp: TempDir = TempDir::new("clear_test").unwrap();
            let config_path = base_tmp.path().join("starship.toml");

            temp_env::with_vars(
                [
                    ("STARSHIP_CONFIG", Some(config_path.as_path())),
                    ("HOME", Some(base_tmp.path())),
                ],
                || {
                    fs::write(&config_path, "").unwrap();

                    let (_iris_dir, ctx) = IrisContext::mock();
                    let generator = StarshipGenerator;
                    let cache_dir = ctx.paths.generators.join(generator.name());

                    fs::create_dir_all(&cache_dir).unwrap();
                    fs::write(cache_dir.join("some_theme.toml"), "data").unwrap();

                    generator.cleanup(&ctx.paths).unwrap();
                    assert!(!cache_dir.exists());
                },
            );
        }

        #[test]
        fn should_remove_theme_for_starship() {
            let base_tmp: TempDir = TempDir::new("remove_test").unwrap();
            let config_path = base_tmp.path().join("starship.toml");
            let theme_name = "test_theme";

            fs::create_dir_all(base_tmp.path()).unwrap();
            fs::write(
                &config_path,
                format!(
                    "palette = \"{}\"\n# [iris:begin:starship]\n[palettes.{}]\n# [iris:end:starship]",
                    theme_name, theme_name
                ),
            )
            .unwrap();

            temp_env::with_vars(
                [
                    ("STARSHIP_CONFIG", Some(config_path.as_path())),
                    ("HOME", Some(base_tmp.path())),
                ],
                || {
                    let (_iris_dir, mut ctx) = IrisContext::mock();
                    ctx.paths.config = config_path.clone();
                    let generator = StarshipGenerator;

                    let cache_file = generator.cache_path(&ctx.paths, theme_name);
                    fs::create_dir_all(cache_file.parent().unwrap()).unwrap();
                    fs::write(&cache_file, "cache content").unwrap();

                    generator.remove_theme(&ctx.paths, theme_name).unwrap();

                    let final_content = fs::read_to_string(&config_path).unwrap();
                    assert!(!final_content.contains(theme_name));
                    assert!(!cache_file.exists());
                },
            );
        }
    }

    /// Integration tests for starship
    mod integration {
        use super::*;
        use crate::skip_if_not_installed;

        #[test]
        fn should_return_health_ok_for_starship() {
            skip_if_not_installed!(StarshipGenerator);

            let config_dir: TempDir = TempDir::new("starship_test").unwrap();
            let config_path = config_dir.path().join("starship.toml");
            fs::write(&config_path, "").unwrap();

            temp_env::with_vars(
                [
                    ("STARSHIP_CONFIG", Some(config_path.as_path())),
                    ("HOME", Some(config_dir.path())),
                ],
                || {
                    let (_iris_dir, mut ctx) = IrisContext::with_templates(vec![(
                        "prompts/starship",
                        "[palettes.{{ theme_name }}]\nbase      = \"{{ bg }}\"",
                    )]);
                    let generator = StarshipGenerator;
                    let theme: Theme = Theme::mock();
                    ctx.state.theme.current_theme = theme.name.clone();
                    let mut activity = ctx.log.step("Test", false).muted();

                    ctx.engine(&theme)
                        .execute_apply(&generator, &mut activity)
                        .unwrap();

                    let status = generator.health_check(&ctx.paths, &theme.name);
                    assert!(status.is_ok(), "Expected Ok, got: {status}");
                },
            );
        }

        #[test]
        fn should_return_health_warning_wrong_palette_for_starship() {
            skip_if_not_installed!(StarshipGenerator);

            let config_dir: TempDir = TempDir::new("starship_test").unwrap();
            let config_path = config_dir.path().join("starship.toml");

            temp_env::with_vars(
                [
                    ("STARSHIP_CONFIG", Some(config_path.as_path())),
                    ("HOME", Some(config_dir.path())),
                ],
                || {
                    let (_iris_dir, mut ctx) = IrisContext::mock();
                    let generator = StarshipGenerator;
                    let theme: Theme = Theme::mock();

                    let mut activity = ctx.log.step("Test", false).muted();
                    ctx.state.theme.current_theme = theme.name.clone();
                    ctx.engine(&theme)
                        .execute_apply(&generator, &mut activity)
                        .unwrap();

                    let content = fs::read_to_string(&config_path).unwrap();
                    let corrupted = content.replace(
                        &format!("palette = \"{}\"", theme.name),
                        "palette = \"wrong\"",
                    );
                    fs::write(&config_path, corrupted).unwrap();

                    let status = generator.health_check(&ctx.paths, &theme.name);

                    assert!(status.is_warning(), "Expected Warning, got: {status}");
                    assert!(status.contains("Theme not imported"));
                },
            );
        }

        #[test]
        fn should_return_health_error_if_config_missing_for_starship() {
            skip_if_not_installed!(StarshipGenerator);

            let (_iris_dir, ctx) = IrisContext::mock();
            let config_path = ctx.paths.config.join("starship_missing.toml");
            let home_dir = ctx.paths.config.parent().unwrap();

            temp_env::with_vars(
                [
                    ("STARSHIP_CONFIG", Some(config_path.as_path())),
                    ("HOME", Some(home_dir)),
                ],
                || {
                    let generator = StarshipGenerator;
                    let status = generator.health_check(&ctx.paths, "any");
                    assert!(status.is_error(), "Expected Error, got: {status}");
                    assert!(status.contains("Configuration file missing"));
                },
            );
        }

        #[test]
        fn should_fix_wrong_palette_name_for_starship() {
            skip_if_not_installed!(StarshipGenerator);

            let (_iris_dir, ctx) = IrisContext::with_templates(vec![(
                "prompts/starship",
                "[palettes.{{ theme_name }}]\nbase      = \"{{ bg }}\"",
            )]);
            let config_path = ctx.paths.config.join("starship.toml");
            let home_dir = ctx.paths.config.parent().unwrap();

            temp_env::with_vars(
                [
                    ("STARSHIP_CONFIG", Some(config_path.as_path())),
                    ("HOME", Some(home_dir)),
                ],
                || {
                    fs::write(
                        &config_path,
                        "palette = \"wrong-theme\"\n# [iris:begin:starship]\n[palettes.melange]\nbg = \"#000000\"\n# [iris:end:starship]",
                    )
                    .unwrap();

                    let generator = StarshipGenerator;
                    let theme: Theme = Theme::mock();

                    let status = generator.health_check(&ctx.paths, &theme.name);
                    assert!(status.is_warning(), "Expected Warning, got: {status}");
                    assert!(status.contains("Theme not imported"));

                    let mut activity = ctx.log.step("Fix", false);
                    let engine = ctx.engine(&theme);
                    engine
                        .execute_fix(&generator, &status, &mut activity)
                        .unwrap();

                    let content = fs::read_to_string(&config_path).unwrap();
                    assert!(content.contains(&format!("palette = \"{}\"", theme.name)));
                    assert!(generator.health_check(&ctx.paths, &theme.name).is_ok());
                },
            );
        }

        #[test]
        fn should_fix_missing_palette_block_for_starship() {
            skip_if_not_installed!(StarshipGenerator);

            let base_tmp: TempDir = TempDir::new("missing_block").unwrap();
            let config_path = base_tmp.path().join("starship.toml");

            temp_env::with_vars(
                [
                    ("STARSHIP_CONFIG", Some(config_path.as_path())),
                    ("HOME", Some(base_tmp.path())),
                ],
                || {
                    let (_iris_dir, ctx) = IrisContext::with_templates(vec![(
                        "prompts/starship",
                        "[palettes.{{ theme_name }}]\nbase      = \"{{ bg }}\"",
                    )]);
                    let generator = StarshipGenerator;
                    let theme: Theme = Theme::mock();

                    fs::write(
                        &config_path,
                        format!(
                            "palette = \"{}\"\n[directory]\nstyle = \"blue\"",
                            theme.name
                        ),
                    )
                    .unwrap();

                    let status = generator.health_check(&ctx.paths, &theme.name);

                    assert!(status.is_warning(), "Expected Warning, got: {status}");

                    let mut activity = ctx.log.step("Fix", false);
                    let engine = ctx.engine(&theme);
                    engine
                        .execute_fix(&generator, &status, &mut activity)
                        .unwrap();

                    let content = fs::read_to_string(&config_path).unwrap();
                    assert!(content.contains(&format!("[palettes.{}]", theme.name.to_lowercase())));
                    assert!(content.contains(&theme.colors.bg));
                    assert!(content.contains("# [iris:begin:starship]"));
                    assert!(content.contains("# [iris:end:starship]"));
                    assert!(generator.health_check(&ctx.paths, &theme.name).is_ok());
                },
            );
        }
    }
}
