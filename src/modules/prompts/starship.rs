use crate::{
    commands::HealthStatus,
    core::IrisContext,
    models::Palette,
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

    fn link_path(&self, ctx: &IrisContext, _theme_name: &str) -> PathBuf {
        if let Some(env_path) = self.env_config_directory() {
            return env_path;
        }

        self.resolve_config_directory(ctx)
            .join(self.target_file_name(""))
    }

    fn env_config_directory(&self) -> Option<PathBuf> {
        env::var("STARSHIP_CONFIG").ok().map(PathBuf::from)
    }

    fn apply(&self, p: &Palette, ctx: &IrisContext) -> Result<()> {
        ctx.log.info(&format!(
            "Writing {} theme to {}",
            utils::capitalize(&p.name).yellow(),
            self.name().bold().cyan(),
        ));
        let config_path: PathBuf = self.link_path(ctx, &p.name);

        self.update_config_file(&config_path, p, ctx)?;

        ctx.log.info(&format!(
            "{} theme applied to {}",
            utils::capitalize(&p.name).yellow(),
            self.name().bold().cyan()
        ));
        Ok(())
    }

    fn build_render_context(&self, p: &Palette) -> tera::Context {
        let mut c = tera::Context::new();
        c.insert("theme_name", &p.name);
        c.insert("bg", &p.bg);
        c.insert("fg", &p.fg);
        c.insert("sel", &p.sel);
        c.insert("line_hl", &p.line_hl);
        c.insert("gutter_fg", &p.gutter_fg);
        c.insert("ansi", &p.ansi);
        c
    }

    fn health_check(&self, ctx: &IrisContext) -> HealthStatus {
        if !self.is_installed() {
            return HealthStatus::Warning("`starship` binary not found".into());
        }

        let config_path: PathBuf = self.link_path(ctx, "");

        if !config_path.exists() {
            return HealthStatus::Error {
                message: "starship.toml not found".into(),
                fix_hint: Some(format!("Create config at {}", config_path.display())),
            };
        }

        let theme: &String = &ctx.state.current_theme;

        if !theme.is_empty() {
            let content: String = fs::read_to_string(&config_path).unwrap_or_default();

            let expected_key = format!("palette = \"{}\"", theme);
            if !content.contains(&expected_key) {
                return HealthStatus::Warning(format!(
                    "`starship` is not using the current palette '{}'",
                    theme
                ));
            }

            let expected_header = format!("[palettes.{}]", theme);
            if !content.contains(&expected_header) {
                return HealthStatus::Error {
                    message: format!("Palette block '{}' missing in config", theme),
                    fix_hint: Some(
                        "Run `iris sync` or `iris health --fix` to inject the palette block".into(),
                    ),
                };
            }
        }

        HealthStatus::Ok
    }

    fn fix(&self, status: &HealthStatus, p: &Palette, ctx: &IrisContext) -> Result<()> {
        match status {
            HealthStatus::Error { .. } | HealthStatus::Warning(_) => {
                ctx.log
                    .step(
                        &format!("Re-applying `{}` configuration...", self.name().bold()),
                        2,
                    )
                    .done(true);

                self.apply(p, &ctx.silent())
            }
            _ => Ok(()),
        }
    }
}

impl StarshipGenerator {
    fn update_config_file(&self, path: &Path, p: &Palette, ctx: &IrisContext) -> Result<()> {
        let render_ctx = self.build_render_context(p);
        let new_palette_block: String = ctx.templater.render(&self.template_path(), &render_ctx)?;

        let content = if path.exists() {
            fs::read_to_string(path)
                .with_context(|| format!("Failed to read `starship` config: {}", path.display()))?
        } else {
            String::new()
        };

        let mut clean_lines: Vec<String> = Vec::new();
        let mut skip_block = false;

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("palette =") {
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

        let mut final_content = format!(
            "palette = \"{}\"\n\n{}",
            p.name,
            clean_lines.join("\n").trim()
        );
        final_content.push_str("\n\n");
        final_content.push_str(&new_palette_block);

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "Failed to create directory for `starship` config: {}",
                    parent.display()
                )
            })?;
        }

        fs::write(path, final_content)
            .with_context(|| format!("Failed to write `starship` config: {}", path.display()))?;

        Ok(())
    }
}

/// Unit-tests for starship generator
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::tests::create_test_context;

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
        let p = Palette::mock();
        let ctx = generator.build_render_context(&p);

        assert_eq!(ctx.get("bg").expect("bg missing").as_str().unwrap(), p.bg);
        assert_eq!(ctx.get("fg").expect("fg missing").as_str().unwrap(), p.fg);

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
        let (tmp_dir, ctx) = create_test_context();
        let generator = StarshipGenerator;
        let p = Palette::mock();
        let config_path = tmp_dir.path().join("starship.toml");

        let initial_content = r##"
    palette = "old_theme"
    [directory]
    style = "blue"

    [palettes.old_theme]
    base = "#000000"

    palette = "duplicate_key"
    "##;
        fs::write(&config_path, initial_content).unwrap();

        temp_env::with_var("STARSHIP_CONFIG", Some(&config_path), || {
            generator.apply(&p, &ctx).expect("Failed to apply");
            let result = fs::read_to_string(&config_path).unwrap();

            let palette_occurrences: Vec<_> = result.matches("palette =").collect();
            assert_eq!(
                palette_occurrences.len(),
                1,
                "Should have exactly one palette key"
            );
            assert!(result.starts_with(&format!("palette = \"{}\"", p.name)));
            assert!(!result.contains("[palettes.old_theme]"));
            assert!(result.contains(&format!("[palettes.{}]", p.name)));
            assert!(result.contains("[directory]"));
            assert!(result.contains("style = \"blue\""));
        });
    }

    #[test]
    fn should_return_health_ok_for_starship() {
        let (tmp_dir, mut ctx) = create_test_context();
        let generator = StarshipGenerator;
        let p = Palette::mock();
        let config_path = tmp_dir.path().join("starship.toml");

        temp_env::with_var("STARSHIP_CONFIG", Some(&config_path), || {
            ctx.state.current_theme = p.name.clone();
            generator.apply(&p, &ctx).unwrap();

            let status = generator.health_check(&ctx);
            assert!(
                matches!(status, HealthStatus::Ok),
                "Expected Ok, got {:?}",
                status
            );
        });
    }

    #[test]
    fn should_return_health_warning_wrong_palette_for_starship() {
        let (tmp_dir, mut ctx) = create_test_context();
        let generator = StarshipGenerator;
        let p = Palette::mock();
        let root = tmp_dir.path();
        let config_path = root.join("starship.toml");

        temp_env::with_var("STARSHIP_CONFIG", Some(&config_path), || {
            ctx.state.current_theme = p.name.clone();
            generator.apply(&p, &ctx).unwrap();

            let content = fs::read_to_string(&config_path).unwrap();
            let corrupted =
                content.replace(&format!("palette = \"{}\"", p.name), "palette = \"wrong\"");
            fs::write(&config_path, corrupted).unwrap();

            let status = generator.health_check(&ctx);
            assert!(
                matches!(&status, HealthStatus::Warning(msg) if msg.contains("not using the current palette")),
                "Expected Warning for wrong palette, got {:?}",
                status
            );
        });
    }

    #[test]
    fn should_return_health_error_missing_palette_block_for_starship() {
        let (tmp_dir, mut ctx) = create_test_context();
        let generator = StarshipGenerator;
        let p = Palette::mock();
        let root = tmp_dir.path();
        let config_path = root.join("starship.toml");

        temp_env::with_var("STARSHIP_CONFIG", Some(&config_path), || {
            ctx.state.current_theme = p.name.clone();
            generator.apply(&p, &ctx).unwrap();

            let content = fs::read_to_string(&config_path).unwrap();
            let header = format!("[palettes.{}]", p.name);
            let lines: Vec<&str> = content
                .lines()
                .filter(|line| !line.trim().starts_with(&header))
                .collect();
            fs::write(&config_path, lines.join("\n")).unwrap();

            let status = generator.health_check(&ctx);
            match status {
                HealthStatus::Error { ref message, .. } => {
                    assert!(
                        message.contains("missing"),
                        "Error message should mention 'missing' block"
                    );
                }
                _ => panic!("Expected Error for missing palette block, got {:?}", status),
            }
        });
    }

    #[test]
    fn should_apply_theme_for_starship() {
        let (tmp_dir, ctx) = create_test_context();
        let generator = StarshipGenerator;
        let p = Palette::mock();
        let config_path = tmp_dir.path().join("starship.toml");

        temp_env::with_var("STARSHIP_CONFIG", Some(&config_path), || {
            fs::write(&config_path, "[directory]\nstyle = \"blue\"\n").unwrap();

            let result = generator.apply(&p, &ctx);
            assert!(result.is_ok());

            let final_content = fs::read_to_string(&config_path).unwrap();

            assert!(final_content.contains(&format!("palette = \"{}\"", p.name)));
            assert!(final_content.contains(&format!("[palettes.{}]", p.name)));
            assert!(final_content.contains("[directory]"));
        });
    }

    #[test]
    fn should_fix_missing_palette_block_for_starship() {
        let (tmp_dir, mut ctx) = create_test_context();
        let generator = StarshipGenerator;
        let p = Palette::mock();
        let root = tmp_dir.path();
        let config_path = root.join("starship.toml");

        temp_env::with_var("STARSHIP_CONFIG", Some(&config_path), || {
            ctx.state.current_theme = p.name.clone();

            fs::write(
                &config_path,
                format!("palette = \"{}\"\n[other_section]\nfoo = \"bar\"", p.name),
            )
            .unwrap();

            let status = generator.health_check(&ctx);
            assert!(
                matches!(status, HealthStatus::Error { ref message, .. } if message.contains("missing")),
                "Expected Error for missing block, got {:?}",
                status
            );
        });
    }

    #[test]
    fn should_fix_wrong_palette_name_for_starship() {
        let (tmp_dir, mut ctx) = create_test_context();
        let generator = StarshipGenerator;
        let p = Palette::mock();
        let root = tmp_dir.path();
        let config_path = root.join("starship.toml");

        temp_env::with_var("STARSHIP_CONFIG", Some(&config_path), || {
            ctx.state.current_theme = p.name.clone();
            fs::write(
                &config_path,
                "palette = \"wrong-theme\"\n[palettes.melange]\nbg = \"#000000\"",
            )
            .unwrap();

            let status = generator.health_check(&ctx);
            assert!(
                matches!(status, HealthStatus::Warning(ref msg) if msg.contains("not using the current palette")),
                "Expected Warning for wrong palette name, got {:?}",
                status
            );
        });
    }
}
