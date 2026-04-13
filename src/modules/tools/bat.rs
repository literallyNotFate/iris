use super::rules::RULES;
use crate::{
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

    fn resolve_config_directory(&self) -> PathBuf {
        Command::new("bat")
            .arg("--config-dir")
            .output()
            .map(|o| PathBuf::from(String::from_utf8_lossy(&o.stdout).trim()).join("themes"))
            .unwrap_or_else(|_| {
                dirs::home_dir()
                    .map(|p| p.join(".config").join("bat").join("themes"))
                    .unwrap_or_else(|| PathBuf::from(".config/bat/themes"))
            })
    }

    fn apply(&self, p: &Palette, ctx: &IrisContext) -> Result<()> {
        let theme_file_name: String = self.target_file_name(&p.name);

        ctx.log.info("Fetching bat configuration directory...");
        let themes_dir: PathBuf = self.resolve_config_directory();
        ctx.log
            .info(&format!("Config found at: {}", themes_dir.display()));

        let cache_theme_path: PathBuf = ctx.paths.cache.join("bat_themes").join(&theme_file_name);
        let link_path: PathBuf = themes_dir.join(&theme_file_name);

        let render_ctx = self.build_render_context(p);
        let content: String = ctx.templater.render(&self.template_path(), &render_ctx)?;

        if let Some(parent) = cache_theme_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create cache directory for {}", self.name()))?;
        }

        fs::write(&cache_theme_path, content.trim())?;

        if !themes_dir.exists() {
            ctx.log.info(&format!(
                "Creating {} config directory...",
                self.name().bold()
            ));
            fs::create_dir_all(&themes_dir)?;
        }

        if link_path.exists() || link_path.is_symlink() {
            fs::remove_file(&link_path)?;
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            ctx.log.info("Linking theme to bat/themes...");
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
        let output = Command::new("bat").arg("cache").arg("--build").output()?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            ctx.log
                .error(&format!("Bat cache build failed with: {}", err.trim()), 2);
            anyhow::bail!("Bat cache build failed");
        }

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

    fn setup_hint(&self) -> Option<String> {
        let bat_config_path: PathBuf = dirs::home_dir()?.join(".cache/iris/bat.conf");

        let env_var: String = std::env::var("BAT_CONFIG_PATH").unwrap_or_default();
        if env_var != bat_config_path.to_string_lossy().as_ref() {
            return Some(format!(
                "Bat theme won't load until you add to your shell config:\n     {}",
                format!("export BAT_CONFIG_PATH=\"{}\"", bat_config_path.display()).yellow()
            ));
        }

        None
    }
}

/// Unit-tests for bat generator
#[cfg(test)]
mod tests {
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
    fn should_generate_setup_hint_for_bat() {
        let generator = BatGenerator;
        let temp_dir: TempDir = TempDir::new("bat_test").unwrap();
        let fake_iris_cache = temp_dir.path().join(".cache/iris/bat.conf");

        temp_env::with_vars(
            vec![("HOME", Some(temp_dir.path())), ("BAT_CONFIG_PATH", None)],
            || {
                let hint = generator.setup_hint();
                assert!(hint.is_some());
                assert!(hint.unwrap().contains("BAT_CONFIG_PATH"));

                temp_env::with_var("BAT_CONFIG_PATH", Some(&fake_iris_cache), || {
                    let hint_after = generator.setup_hint();
                    assert!(
                        hint_after.is_none(),
                        "Hint should disappear when env var matches"
                    );
                });
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
