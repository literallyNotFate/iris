use crate::{
    core::{IrisPaths, Templater},
    guards::FsRollbackGuard,
    log::Task,
    models::{HealthStatus, Theme},
    modules::{Generator, GeneratorType},
    utils,
};
use anyhow::{Context, Result};
use colored::Colorize;
use std::{
    env, fs,
    path::{Path, PathBuf},
};

/// Config generator for starship
pub struct StarshipGenerator;

impl Generator for StarshipGenerator {
    fn name(&self) -> &str {
        "starship"
    }

    fn generator_type(&self) -> GeneratorType {
        GeneratorType::Prompt
    }

    fn target_file_name(&self, _theme: &str) -> String {
        "starship.toml".into()
    }

    fn resolve_config_directory(&self, paths: &IrisPaths) -> PathBuf {
        paths.config.clone()
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

    fn env_config_directory(&self) -> Option<PathBuf> {
        env::var("STARSHIP_CONFIG").ok().map(PathBuf::from)
    }

    fn apply(
        &self,
        theme: &Theme,
        paths: &IrisPaths,
        templater: &Templater,
        task: &mut Task,
    ) -> Result<()> {
        task.info(&format!(
            "Generating {} theme for {}...",
            theme.name.yellow(),
            self.name().bold().cyan()
        ));

        let cache_file: PathBuf = self.cache_path(paths, &theme.name.to_lowercase());
        let link_path: PathBuf = self.link_path(paths, &theme.name.to_lowercase());
        let backup_path: PathBuf = link_path.with_extension("toml.bak");

        let render_ctx = self.build_render_context(theme);
        let palette_block: String = templater
            .render(&self.template_path(), &render_ctx)
            .context("Failed to render starship palette block")?;

        if let Some(parent) = cache_file.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&cache_file, &palette_block).with_context(|| {
            format!("Failed to write palette cache to {}", cache_file.display())
        })?;

        let rollback_guard = FsRollbackGuard::new(link_path.clone(), backup_path);

        self.update_config_file(&link_path, &link_path, theme, &palette_block)?;
        rollback_guard.commit();

        task.info(&format!(
            "{} theme applied to {}",
            theme.name.yellow(),
            self.name().bold().cyan()
        ));
        Ok(())
    }

    fn build_render_context(&self, theme: &Theme) -> tera::Context {
        let mut c = tera::Context::new();
        c.insert("theme_name", &theme.name.to_lowercase());
        c.insert("bg", &theme.colors.bg);
        c.insert("fg", &theme.colors.fg);
        c.insert("sel", &theme.colors.sel);
        c.insert("line_hl", &theme.colors.line_hl);
        c.insert("gutter_fg", &theme.colors.gutter_fg);
        c.insert("ansi", &theme.colors.ansi);
        c
    }

    fn health_check(&self, paths: &IrisPaths, theme: &str) -> HealthStatus {
        if !self.is_installed() {
            return HealthStatus::Warning("`starship` binary not found".into());
        }

        let config_path: PathBuf = self.link_path(paths, "");
        let file_status = HealthStatus::check_file(&config_path, "starship.toml");

        if file_status.is_error() {
            return HealthStatus::error(
                "starship.toml not found",
                Some(format!("Create config at {}", config_path.display())),
            );
        }

        if !theme.is_empty() {
            let content: String = fs::read_to_string(&config_path).unwrap_or_default();
            let expected_key: String = format!("palette = \"{}\"", theme);
            if !content.contains(&expected_key) {
                return HealthStatus::Warning(format!(
                    "`starship` is not using the current palette '{theme}'"
                ));
            }

            let expected_header: String = format!("[palettes.{}]", theme);
            if !content.contains(&expected_header) {
                return HealthStatus::error(
                    format!("Palette block '{theme}' missing in config"),
                    Some("Run `iris sync` or `iris health --fix` to inject the palette block"),
                );
            }
        }

        HealthStatus::Ok
    }

    fn fix(
        &self,
        status: &HealthStatus,
        theme: &Theme,
        paths: &IrisPaths,
        templater: &Templater,
        task: &mut Task,
    ) -> Result<()> {
        if status.is_error() || status.is_warning() {
            return task.log.action(
                &format!("Re-applied `{}` configuration", self.name().bold()),
                || self.apply(theme, paths, templater, &mut task.as_quiet()),
            );
        }

        Ok(())
    }

    fn clear(&self, paths: &IrisPaths) -> Result<()> {
        let name: &str = self.name();
        let config_path: PathBuf = self.link_path(paths, "");

        if config_path.exists() {
            let backup_path: PathBuf = config_path.with_extension("toml.bak");
            let rollback_guard = FsRollbackGuard::new(config_path.clone(), backup_path);

            self.remove_palette_block(&config_path)
                .context("Failed to clean up starship.toml during clear")?;
            rollback_guard.commit();
        }

        let gen_cache_dir: PathBuf = paths.generators.join(name);
        if gen_cache_dir.exists() {
            fs::remove_dir_all(&gen_cache_dir).with_context(|| {
                format!(
                    "Failed to remove generator directory for {}: {}",
                    name,
                    utils::pretty_path(&gen_cache_dir)
                )
            })?;
        }

        Ok(())
    }

    fn remove_theme(&self, paths: &IrisPaths, theme_name: &str) -> Result<()> {
        let theme_name_lower: String = theme_name.to_lowercase();
        let config_path: PathBuf = self.link_path(paths, "");

        if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            let active_palette_line = format!("palette = \"{}\"", theme_name_lower);

            if content.contains(&active_palette_line) {
                self.remove_palette_block(&config_path)
                    .context("Failed to remove active palette from starship.toml")?;
            }
        }

        let theme_cache_file: PathBuf = self.cache_path(paths, &theme_name_lower);
        if theme_cache_file.exists() {
            fs::remove_file(&theme_cache_file).with_context(|| {
                format!(
                    "Failed to remove theme cache: {}",
                    theme_cache_file.display()
                )
            })?;
        }
        Ok(())
    }
}

impl StarshipGenerator {
    fn clean_config_content(&self, content: &str) -> Vec<String> {
        let mut clean_lines: Vec<String> = Vec::new();
        let mut skip_block = false;

        for line in content.lines() {
            let trimmed: &str = line.trim();
            if trimmed.replace(" ", "").starts_with("palette=") {
                continue;
            }

            if trimmed.starts_with("[palettes.") {
                skip_block = true;
                continue;
            }

            if skip_block && trimmed.starts_with('[') && !trimmed.starts_with("[palettes.") {
                skip_block = false;
            }

            if !skip_block {
                clean_lines.push(line.to_string());
            }
        }

        clean_lines
    }

    fn update_config_file(
        &self,
        target_path: &Path,
        write_path: &Path,
        theme: &Theme,
        palette_block: &str,
    ) -> Result<()> {
        let content = if target_path.exists() {
            fs::read_to_string(target_path).with_context(|| {
                format!(
                    "Failed to read `starship` config: {}",
                    target_path.display()
                )
            })?
        } else {
            String::new()
        };

        let clean_lines = self.clean_config_content(&content);
        let mut final_content = String::new();
        final_content.push_str(&format!("palette = \"{}\"\n\n", theme.name.to_lowercase()));

        let body = clean_lines.join("\n").trim().to_string();
        if !body.is_empty() {
            final_content.push_str(&body);
            final_content.push_str("\n\n");
        }

        final_content.push_str(palette_block);
        if let Some(parent) = write_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(write_path, final_content.trim()).with_context(|| {
            format!(
                "Failed to write `starship` config: {}",
                write_path.display()
            )
        })?;

        Ok(())
    }

    pub fn remove_palette_block(&self, target_path: &Path) -> Result<()> {
        if !target_path.exists() {
            return Ok(());
        }

        let content: String = fs::read_to_string(target_path)?;
        let clean_lines: Vec<String> = self.clean_config_content(&content);

        fs::write(target_path, clean_lines.join("\n").trim()).with_context(|| {
            format!(
                "Failed to write cleaned `starship` config: {}",
                target_path.display()
            )
        })?;

        Ok(())
    }
}

/// Unit-tests for starship generator
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::tests::create_test_context;
    use tempdir::TempDir;

    #[test]
    fn should_return_starship_metadata() {
        let generator = StarshipGenerator;
        assert_eq!(generator.name(), "starship");
        assert_eq!(generator.generator_type(), GeneratorType::Prompt);
        assert_eq!(generator.target_file_name("any"), "starship.toml");
    }

    #[test]
    fn should_build_valid_render_context() {
        let generator = StarshipGenerator;
        let theme: Theme = Theme::mock();
        let ctx = generator.build_render_context(&theme);

        assert_eq!(
            ctx.get("bg").expect("bg missing").as_str().unwrap(),
            theme.colors.bg
        );
        assert_eq!(
            ctx.get("fg").expect("fg missing").as_str().unwrap(),
            theme.colors.fg
        );

        let ansi = ctx
            .get("ansi")
            .expect("ansi array missing")
            .as_array()
            .expect("ansi should be an array");
        assert!(ansi.len() >= 16);
        assert!(ctx.contains_key("gutter_fg"));
        assert!(ctx.contains_key("sel"));
        assert!(ctx.contains_key("line_hl"));
        assert!(ctx.contains_key("theme_name"));
    }

    #[test]
    fn should_clean_and_inject_correctly() {
        let (_iris_dir, ctx) = create_test_context();
        let config_path = ctx.paths.config.join("starship.toml");
        let home_dir = ctx.paths.config.parent().unwrap();

        let initial_content = r##"
    palette = "old_theme"
    [directory]
    style = "blue"

    [palettes.old_theme]
    base = "#000000"
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

                let mut task = ctx.log.step("Test", false).as_quiet();
                generator
                    .apply(&theme, &ctx.paths, &ctx.templater, &mut task)
                    .unwrap();

                let result = fs::read_to_string(&config_path).unwrap();
                let palette_occurrences: Vec<_> = result.matches("palette =").collect();

                assert_eq!(palette_occurrences.len(), 1);
                assert!(result.contains(&format!("palette = \"{}\"", theme.name)));
                assert!(!result.contains("[palettes.old_theme]"));
                assert!(result.contains(&format!("[palettes.{}]", theme.name)));
                assert!(result.contains("[directory]"));
            },
        );
    }

    #[test]
    fn should_return_health_ok_for_starship() {
        let (_iris_dir, mut ctx) = create_test_context();
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

                ctx.state.current_theme = theme.name.clone();
                let mut task = ctx.log.step("Test", false).as_quiet();
                generator
                    .apply(&theme, &ctx.paths, &ctx.templater, &mut task)
                    .unwrap();

                let status = generator.health_check(&ctx.paths, &theme.name);
                assert!(status.is_ok(), "Expected Ok, got: {status}");
            },
        );
    }

    #[test]
    fn should_return_health_warning_wrong_palette_for_starship() {
        let (_iris_dir, mut ctx) = create_test_context();
        let config_path = ctx.paths.config.join("starship.toml");
        let home_dir = ctx.paths.config.parent().unwrap();

        temp_env::with_vars(
            [
                ("STARSHIP_CONFIG", Some(config_path.as_path())),
                ("HOME", Some(home_dir)),
            ],
            || {
                let generator = StarshipGenerator;
                let theme: Theme = Theme::mock();

                let mut task = ctx.log.step("Test", false).as_quiet();
                ctx.state.current_theme = theme.name.clone();
                generator
                    .apply(&theme, &ctx.paths, &ctx.templater, &mut task)
                    .unwrap();

                let content = fs::read_to_string(&config_path).unwrap();
                let corrupted = content.replace(
                    &format!("palette = \"{}\"", theme.name),
                    "palette = \"wrong\"",
                );
                fs::write(&config_path, corrupted).unwrap();

                let status = generator.health_check(&ctx.paths, &theme.name);

                assert!(status.is_warning(), "Expected Warning, got: {status}");
                assert!(status.contains("not using the current palette"));
            },
        );
    }

    #[test]
    fn should_return_health_error_if_config_missing() {
        let (_iris_dir, ctx) = create_test_context();
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
                assert!(status.contains("not found"));
            },
        );
    }

    #[test]
    fn should_apply_theme_for_starship() {
        let (_iris_dir, ctx) = create_test_context();
        let config_path = ctx.paths.config.join("starship.toml");
        let home_dir = ctx.paths.config.parent().unwrap();

        temp_env::with_vars(
            [
                ("STARSHIP_CONFIG", Some(config_path.as_path())),
                ("HOME", Some(home_dir)),
            ],
            || {
                fs::write(&config_path, "[directory]\nstyle = \"blue\"\n").unwrap();

                let generator = StarshipGenerator;
                let theme: Theme = Theme::mock();

                let mut task = ctx.log.step("Test", false).as_quiet();
                let result = generator.apply(&theme, &ctx.paths, &ctx.templater, &mut task);
                assert!(result.is_ok());

                let final_content = fs::read_to_string(&config_path).unwrap();

                assert!(final_content.contains(&format!("palette = \"{}\"", theme.name)));
                assert!(final_content.contains(&format!("[palettes.{}]", theme.name)));
                assert!(final_content.contains("[directory]"));
            },
        );
    }

    #[test]
    fn should_fix_wrong_palette_name_for_starship() {
        let (_iris_dir, ctx) = create_test_context();
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
                    "palette = \"wrong-theme\"\n[palettes.melange]\nbg = \"#000000\"",
                )
                .unwrap();

                let generator = StarshipGenerator;
                let theme: Theme = Theme::mock();

                let status = generator.health_check(&ctx.paths, &theme.name);

                assert!(status.is_warning(), "Expected Warning, got: {status}");
                assert!(status.contains("not using the current palette"));

                let mut task = ctx.log.step("Fix", false);
                generator
                    .fix(&status, &theme, &ctx.paths, &ctx.templater, &mut task)
                    .unwrap();

                let content = fs::read_to_string(&config_path).unwrap();
                assert!(content.contains(&format!("palette = \"{}\"", theme.name)));
                assert!(generator.health_check(&ctx.paths, &theme.name).is_ok());
            },
        );
    }

    #[test]
    fn should_fix_missing_palette_block_for_starship() {
        let base_tmp: TempDir = TempDir::new("missing_block").unwrap();
        let config_path = base_tmp.path().join("starship.toml");

        temp_env::with_vars(
            [
                ("STARSHIP_CONFIG", Some(config_path.as_path())),
                ("HOME", Some(base_tmp.path())),
            ],
            || {
                let (_iris_dir, ctx) = create_test_context();
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

                assert!(status.is_error(), "Expected Error, got: {status}");
                assert!(status.contains("missing"));

                let mut task = ctx.log.step("Fix", false);
                generator
                    .fix(&status, &theme, &ctx.paths, &ctx.templater, &mut task)
                    .unwrap();

                let content = fs::read_to_string(&config_path).unwrap();
                assert!(content.contains(&format!("[palettes.{}]", theme.name)));
                assert!(content.contains(&theme.colors.bg));
                assert!(generator.health_check(&ctx.paths, &theme.name).is_ok());
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

                let (_iris_dir, ctx) = create_test_context();
                let generator = StarshipGenerator;
                let cache_dir = ctx.paths.generators.join(generator.name());

                fs::create_dir_all(&cache_dir).unwrap();
                fs::write(cache_dir.join("some_theme.toml"), "data").unwrap();

                generator.clear(&ctx.paths).unwrap();
                assert!(!cache_dir.exists());
            },
        );
    }

    #[test]
    fn should_remove_theme_for_starship() {
        let base_tmp: TempDir = TempDir::new("remove_test").unwrap();
        let config_path = base_tmp.path().join("starship.toml");
        let theme_name = "test_theme";

        temp_env::with_vars(
            [
                ("STARSHIP_CONFIG", Some(config_path.as_path())),
                ("HOME", Some(base_tmp.path())),
            ],
            || {
                fs::write(
                    &config_path,
                    format!("palette = \"{}\"\n[palettes.{}]\n", theme_name, theme_name),
                )
                .unwrap();

                let (_iris_dir, ctx) = create_test_context();
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
