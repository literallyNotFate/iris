use super::rules::RULES;
use crate::{
    commands::HealthStatus,
    core::IrisContext,
    models::Palette,
    modules::{Generator, GeneratorType},
    utils::{self},
};
use anyhow::{Context, Result};
use colored::Colorize;
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

/// Config generator for bat
pub struct BatGenerator;

impl Generator for BatGenerator {
    fn name(&self) -> &str {
        "bat"
    }

    fn generator_type(&self) -> GeneratorType {
        GeneratorType::Tool
    }

    fn target_file_name(&self, theme: &str) -> String {
        format!("{}.tmTheme", theme)
    }

    fn resolve_config_directory(&self, ctx: &IrisContext) -> PathBuf {
        let config_base: PathBuf = ctx
            .paths
            .config
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| ctx.paths.config.clone());

        config_base.join(self.name()).join("themes")
    }

    fn apply(&self, p: &Palette, ctx: &IrisContext) -> Result<()> {
        ctx.log.info(&format!(
            "Generating {} theme for {}...",
            utils::capitalize(&p.name).yellow(),
            self.name().bold().cyan()
        ));

        let cache_theme: PathBuf = self.ensure_theme_cache(p, ctx)?;
        let link_path: PathBuf = self.link_path(ctx, &p.name);

        ctx.log.info(&format!(
            "Linking {} theme to {}...",
            self.name().bold(),
            utils::pretty_path(&link_path).magenta(),
        ));
        self.ensure_symlink(&cache_theme, &link_path, ctx)?;

        self.ensure_config(p, ctx)?;
        self.rebuild_bat_cache(ctx)?;

        ctx.log.info(&format!(
            "{} theme applied to {}",
            utils::capitalize(&p.name).yellow(),
            self.name().bold().cyan()
        ));
        Ok(())
    }

    fn build_render_context(&self, p: &Palette) -> tera::Context {
        let mut c = tera::Context::new();
        let fix = |h: &str| -> String {
            let hex = h.trim_start_matches('#');
            format!("#{}", hex)
        };

        c.insert("theme_name", &utils::capitalize(&p.name));
        c.insert("bg", &fix(&p.bg));
        c.insert("fg", &fix(&p.fg));
        c.insert("sel", &fix(&p.sel));
        c.insert("line", &fix(&p.line_hl));

        let processed_rules: Vec<serde_json::Value> = RULES
            .iter()
            .map(|r| {
                let color = match r.color_key {
                    "keyword" => &p.keyword,
                    "func" => &p.func,
                    "type_name" => &p.type_name,
                    "string" => &p.string,
                    "operator" => &p.operator,
                    "number" => &p.number,
                    "comment" => &p.comment,
                    _ => &p.fg,
                };

                let style = if r.style.is_empty() || r.style == "normal" {
                    None
                } else {
                    Some(r.style)
                };

                serde_json::json!({
                    "name": r.name,
                    "scope": r.scope,
                    "style": style,
                    "foreground": fix(color),
                })
            })
            .collect();

        c.insert("rules", &processed_rules);
        c
    }

    fn health_check(&self, ctx: &IrisContext) -> HealthStatus {
        if !self.is_installed() {
            return HealthStatus::Warning("bat binary not found".into());
        }

        let expected_env: PathBuf = ctx.paths.generators.join(self.name()).join("bat.conf");
        let current_env: String = env::var("BAT_CONFIG_PATH").unwrap_or_default();

        if current_env != expected_env.to_string_lossy() {
            return HealthStatus::Error {
                message: "BAT_CONFIG_PATH is not set correctly".into(),
                fix_hint: Some(format!(
                    "Add 'export BAT_CONFIG_PATH=\"{}\"' to your shell config",
                    expected_env.display()
                )),
            };
        }

        let theme: &String = &ctx.state.current_theme;

        if !theme.is_empty() {
            let link = self.link_path(ctx, theme);
            if !link.exists() {
                return HealthStatus::Error {
                    message: format!(
                        "Theme file '{}.tmTheme' is missing in bat themes directory",
                        theme
                    ),
                    fix_hint: Some("Run `iris sync` to relink and rebuild cache".into()),
                };
            }
        }

        HealthStatus::Ok
    }

    fn fix(&self, status: &HealthStatus, p: &Palette, ctx: &IrisContext) -> Result<()> {
        match status {
            HealthStatus::Error { message, .. } => {
                if message.contains("missing") || message.contains("not linked") {
                    ctx.log.step("Restoring bat theme symlink...", 2).done(true);

                    let cache = self.cache_path(ctx, &p.name);
                    let link = self.link_path(ctx, &p.name);
                    self.ensure_symlink(&cache, &link, &ctx.silent())?;
                }

                self.apply(p, &ctx.silent())
            }
            HealthStatus::Warning(msg) if msg.contains("cache is older") => {
                let mut t = ctx.log.step("Rebuilding bat theme cache...", 2);
                self.rebuild_bat_cache(ctx)?;
                t.done(true);
                Ok(())
            }
            _ => {
                ctx.log
                    .step("Re-applying bat configuration...", 2)
                    .done(true);
                self.apply(p, &ctx.silent())
            }
        }
    }
}

impl BatGenerator {
    fn ensure_theme_cache(&self, p: &Palette, ctx: &IrisContext) -> Result<PathBuf> {
        let cache_path = self.cache_path(ctx, &p.name);
        let render_ctx = self.build_render_context(p);
        let content = ctx.templater.render(&self.template_path(), &render_ctx)?;

        fs::create_dir_all(cache_path.parent().unwrap())?;
        fs::write(&cache_path, content.trim())?;
        Ok(cache_path)
    }

    fn ensure_symlink(&self, target: &Path, link: &Path, _ctx: &IrisContext) -> Result<()> {
        if link.exists() || link.is_symlink() {
            fs::remove_file(link)?;
        }
        fs::create_dir_all(link.parent().unwrap())?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            symlink(target, link)
                .with_context(|| format!("Failed to create symlink {:?} -> {:?}", target, link))?;
        }
        Ok(())
    }

    fn ensure_config(&self, p: &Palette, ctx: &IrisContext) -> Result<()> {
        let theme_name = utils::capitalize(&p.name);

        let config_content = format!(
            "--theme=\"{name}\"\n--style=\"numbers,changes\"\n--color=\"always\"\n",
            name = theme_name
        );

        let generator_dir: PathBuf = ctx.paths.generators.join(self.name());
        let config_path: PathBuf = generator_dir.join("bat.conf");

        fs::create_dir_all(&generator_dir)?;
        fs::write(config_path, config_content)?;
        Ok(())
    }

    fn rebuild_bat_cache(&self, ctx: &IrisContext) -> Result<()> {
        ctx.log.info("Rebuilding bat cache...");
        Command::new("bat").arg("cache").arg("--build").output()?;
        Ok(())
    }
}

/// Unit-tests for bat generator
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::tests::create_test_context;
    use temp_env;

    #[test]
    fn should_return_bat_metadata() {
        let generator = BatGenerator;
        assert_eq!(generator.name(), "bat");
        assert_eq!(generator.generator_type(), GeneratorType::Tool);
        assert_eq!(generator.target_file_name("nord"), "nord.tmTheme");
    }

    #[test]
    fn should_build_valid_render_context() {
        let generator = BatGenerator;
        let p = Palette::mock();
        let context = generator.build_render_context(&p);
        let data = context.into_json();

        assert_eq!(data["theme_name"], utils::capitalize(&p.name));
        assert!(data["bg"].as_str().unwrap().starts_with('#'));
        assert!(data["fg"].as_str().unwrap().starts_with('#'));

        let rules = data["rules"].as_array().expect("Rules should be an array");
        assert!(!rules.is_empty(), "Rules array should not be empty");

        let first_rule = &rules[0];
        assert!(first_rule["name"].is_string());
        assert!(first_rule["scope"].is_string());
        assert!(first_rule["foreground"].as_str().unwrap().starts_with('#'));
    }

    #[test]
    fn should_return_health_ok_for_bat() {
        let (_, mut ctx) = create_test_context();
        let generator = BatGenerator;
        let p = Palette::mock();

        ctx.state.current_theme = p.name.clone();
        generator.apply(&p, &ctx).unwrap();
        let expected_config = ctx.paths.generators.join(generator.name()).join("bat.conf");

        temp_env::with_var("BAT_CONFIG_PATH", Some(expected_config), || {
            let status = generator.health_check(&ctx);
            assert!(matches!(status, HealthStatus::Ok));
        });
    }

    #[test]
    fn should_return_health_error_bad_env_for_bat() {
        let (_tmp_dir, ctx) = create_test_context();
        let generator = BatGenerator;

        temp_env::with_var("BAT_CONFIG_PATH", Some("/wrong/path/to/bat/config"), || {
            let status = generator.health_check(&ctx);
            match status {
                HealthStatus::Error { message, .. } => {
                    assert!(message.contains("BAT_CONFIG_PATH"));
                }
                _ => panic!("Expected env error because BAT_CONFIG_PATH points to nowhere"),
            }
        });
    }

    #[test]
    fn should_apply_theme_for_bat() {
        let (_, ctx) = create_test_context();
        let generator = BatGenerator;
        let p = Palette::mock();

        let expected_file_name: String = generator.target_file_name(&p.name);
        let expected_theme_name: String = utils::capitalize(&p.name);

        let result = generator.apply(&p, &ctx);
        assert!(result.is_ok(), "Apply failed: {:?}", result.err());

        let cache_theme_path: PathBuf = ctx.paths.generators.join("bat").join(&expected_file_name);
        let bat_conf_path: PathBuf = ctx.paths.generators.join("bat").join("bat.conf");

        assert!(
            cache_theme_path.exists(),
            "Theme file should exist at {:?}",
            cache_theme_path
        );
        assert!(bat_conf_path.exists(), "bat.conf should exist");

        let conf_content = fs::read_to_string(bat_conf_path).unwrap();
        assert!(
            conf_content.contains(&format!("--theme=\"{}\"", expected_theme_name)),
            "Config should contain capitalized theme name: {}",
            expected_theme_name
        );

        let xml_content = fs::read_to_string(cache_theme_path).unwrap();
        assert!(xml_content.contains("<plist"));
        assert!(xml_content.contains(&expected_theme_name));
    }

    #[test]
    fn should_fix_missing_link_for_bat() {
        let (_, ctx) = create_test_context();
        let generator = BatGenerator;
        let p = Palette::mock();

        generator.apply(&p, &ctx).unwrap();
        let link = generator.link_path(&ctx, &p.name);
        fs::remove_file(&link).unwrap();

        let status = generator.health_check(&ctx);
        assert!(matches!(status, HealthStatus::Error { .. }));

        generator.fix(&status, &p, &ctx.silent()).unwrap();
        assert!(link.exists());
    }
}
