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
use std::{fs, path::PathBuf, process::Command};

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

    fn cache_path(&self, ctx: &IrisContext, theme_name: &str) -> PathBuf {
        ctx.paths
            .cache
            .join("bat_themes")
            .join(self.target_file_name(theme_name))
    }

    fn link_path(&self, theme_name: &str) -> PathBuf {
        self.resolve_config_directory()
            .join(self.target_file_name(theme_name))
    }

    fn resolve_config_directory(&self) -> PathBuf {
        dirs::home_dir()
            .map(|p| p.join(".config").join("bat").join("themes"))
            .unwrap_or_else(|| PathBuf::from(".config/bat/themes"))
    }

    fn apply(&self, p: &Palette, ctx: &IrisContext) -> Result<()> {
        let theme_file_name: String = self.target_file_name(&p.name);
        let cache_theme_path: PathBuf = self.cache_path(ctx, &p.name);
        let themes_dir: PathBuf = self.resolve_config_directory();
        let link_path: PathBuf = themes_dir.join(&theme_file_name);

        let render_ctx = self.build_render_context(p);
        let content: String = ctx.templater.render(&self.template_path(), &render_ctx)?;

        fs::create_dir_all(cache_theme_path.parent().unwrap())?;
        if !themes_dir.exists() {
            fs::create_dir_all(&themes_dir)?;
        }

        fs::write(&cache_theme_path, content.trim())?;

        if link_path.exists() || link_path.is_symlink() {
            fs::remove_file(&link_path)?;
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            ctx.log.info(&format!(
                "Linking {} theme to {}...",
                self.name().bold(),
                utils::pretty_path(&cache_theme_path).cyan(),
            ));
            symlink(&cache_theme_path, &link_path).with_context(|| {
                format!("Failed to link {:?} -> {:?}", link_path, cache_theme_path)
            })?;
        }

        let bat_config: String = format!(
            "--theme=\"{name}\"\n--style=\"numbers,changes\"\n--color=\"always\"\n",
            name = utils::capitalize(&p.name)
        );
        fs::write(ctx.paths.cache.join("bat.conf"), bat_config)?;

        ctx.log.info("Rebuilding bat cache...");
        Command::new("bat").arg("cache").arg("--build").output()?;

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

        let expected_env: PathBuf = ctx.paths.cache.join("bat.conf");
        let current_env: String = std::env::var("BAT_CONFIG_PATH").unwrap_or_default();

        if current_env != expected_env.to_string_lossy() {
            return HealthStatus::Error {
                message: "BAT_CONFIG_PATH is not set correctly".into(),
                fix_hint: Some(format!(
                    "Add 'export BAT_CONFIG_PATH=\"{}\"' to shell config",
                    expected_env.display()
                )),
            };
        }

        let theme: &String = &ctx.state.current_theme;

        if !theme.is_empty() {
            let link = self.link_path(theme);
            if !link.exists() {
                return HealthStatus::Error {
                    message: "Theme file is not linked to bat themes directory".into(),
                    fix_hint: Some("Run `iris sync` to relink and rebuild cache".into()),
                };
            }

            let bat_cache_dir: PathBuf = Command::new("bat")
                .arg("--cache-dir")
                .output()
                .map(|o| PathBuf::from(String::from_utf8_lossy(&o.stdout).trim()))
                .unwrap_or_default();

            if let (Ok(m_link), Ok(m_cache)) = (fs::metadata(&link), fs::metadata(bat_cache_dir)) {
                if m_link.modified().unwrap_or(m_cache.modified().unwrap())
                    > m_cache.modified().unwrap()
                {
                    return HealthStatus::Warning(
                        "Bat cache is older than the theme. Rebuild might be needed.".into(),
                    );
                }
            }
        }

        HealthStatus::Ok
    }
}

/// Unit-tests for bat generator
#[cfg(test)]
mod tests {
    use std::{thread, time::Duration};

    use super::*;
    use crate::test_utils::create_test_context;
    use temp_env;
    use tempdir::TempDir;

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
    fn should_resolve_config_directory_fallback_for_bat() {
        let generator = BatGenerator;
        let temp_dir: TempDir = TempDir::new("bat_test").unwrap();

        temp_env::with_var("HOME", Some(temp_dir.path()), || {
            let path = generator.resolve_config_directory();
            assert!(path.to_string_lossy().contains(".config/bat/themes"));
        });
    }

    #[test]
    fn bat_health_ok() {
        let (tmp_dir, mut ctx) = create_test_context();
        let generator = BatGenerator;
        let p = Palette::mock();
        let root = tmp_dir.path();

        let bat_conf_path = ctx.paths.cache.join("bat.conf");
        let bat_cache_dir = root.join(".cache/bat");
        let themes_bin = bat_cache_dir.join("themes.bin");

        temp_env::with_vars(
            vec![
                ("HOME", Some(root.to_str().unwrap())),
                (
                    "XDG_CACHE_HOME",
                    Some(root.join(".cache").to_str().unwrap()),
                ),
                ("BAT_CONFIG_PATH", Some(bat_conf_path.to_str().unwrap())),
            ],
            || {
                ctx.state.current_theme = p.name.clone();
                generator.apply(&p, &ctx).unwrap();

                thread::sleep(Duration::from_millis(10));
                fs::create_dir_all(&bat_cache_dir).unwrap();
                fs::write(&themes_bin, "dummy binary cache content").unwrap();

                let status = generator.health_check(&ctx);

                assert!(
                    matches!(status, HealthStatus::Ok),
                    "Expected HealthStatus::Ok, but got {:?}",
                    status
                );
            },
        );
    }

    #[test]
    fn bat_health_error_bad_env() {
        let (tmp_dir, ctx) = create_test_context();
        let generator = BatGenerator;
        let root = tmp_dir.path();

        temp_env::with_vars(
            vec![
                ("HOME", Some(root.to_str().unwrap())),
                ("BAT_CONFIG_PATH", Some("/wrong/path")),
            ],
            || {
                let status = generator.health_check(&ctx);
                match status {
                    HealthStatus::Error { message, .. } => {
                        assert!(message.contains("BAT_CONFIG_PATH"));
                    }
                    _ => panic!("Expected env error"),
                }
            },
        );
    }

    #[test]
    fn bat_health_warning_stale_cache() {
        let (tmp_dir, mut ctx) = create_test_context();
        let generator = BatGenerator;
        let p = Palette::mock();
        let root = tmp_dir.path();

        temp_env::with_vars(
            vec![
                ("HOME", Some(root.to_str().unwrap())),
                (
                    "XDG_CACHE_HOME",
                    Some(root.join(".cache").to_str().unwrap()),
                ),
                (
                    "BAT_CONFIG_PATH",
                    Some(ctx.paths.cache.join("bat.conf").to_str().unwrap()),
                ),
            ],
            || {
                ctx.state.current_theme = p.name.clone();
                generator.apply(&p, &ctx).unwrap();

                thread::sleep(Duration::from_millis(100));

                let link_path = generator.link_path(&p.name);
                fs::write(&link_path, "force new mtime content").unwrap();

                let status = generator.health_check(&ctx);

                assert!(
                    matches!(&status, HealthStatus::Warning(msg) if msg.contains("cache is older")),
                    "Expected warning about stale cache, got: {:?}",
                    status
                );
            },
        );
    }

    #[test]
    fn should_apply_theme_for_bat() {
        if which::which("bat").is_err() {
            return;
        }

        let (tmp_dir, ctx) = create_test_context();
        let generator = BatGenerator;
        let p = Palette::mock();

        let expected_file_name: String = generator.target_file_name(&p.name);
        let expected_theme_name: String = utils::capitalize(&p.name);

        temp_env::with_var("HOME", Some(tmp_dir.path()), || {
            let result = generator.apply(&p, &ctx);
            assert!(result.is_ok(), "Apply failed: {:?}", result.err());

            let cache_theme_path: PathBuf =
                ctx.paths.cache.join("bat_themes").join(&expected_file_name);
            let bat_conf_path: PathBuf = ctx.paths.cache.join("bat.conf");

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
        });
    }
}
