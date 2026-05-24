use super::rules::RULES;
use crate::{
    core::{IrisPaths, Templater},
    log::{Reporter, Task},
    models::{HealthStatus, Palette},
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

    fn resolve_config_directory(&self, paths: &IrisPaths) -> PathBuf {
        let config_base: PathBuf = paths
            .config
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| paths.config.clone());

        config_base.join(self.name()).join("themes")
    }

    fn apply(
        &self,
        p: &Palette,
        paths: &IrisPaths,
        templater: &Templater,
        task: &mut Task,
    ) -> Result<()> {
        task.info(&format!(
            "Generating {} theme for {}...",
            utils::capitalize(&p.name).yellow(),
            self.name().bold().cyan()
        ));

        let cache_theme: PathBuf = self.ensure_theme_cache(p, paths, templater)?;
        let link_path: PathBuf = self.link_path(paths, &p.name);

        task.info(&format!(
            "Linking {} theme to {}...",
            self.name().bold(),
            utils::pretty_path(&link_path).magenta(),
        ));
        self.ensure_symlink(&cache_theme, &link_path)?;

        self.ensure_config(p, paths)?;
        self.rebuild_bat_cache(task)?;

        task.info(&format!(
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

    fn health_check(&self, paths: &IrisPaths, theme: &str) -> HealthStatus {
        if !self.is_installed() {
            return HealthStatus::Warning("`bat` binary not found".into());
        }

        let expected_env: PathBuf = paths.generators.join(self.name()).join("bat.conf");
        let current_env: String = env::var("BAT_CONFIG_PATH").unwrap_or_default();

        if current_env != expected_env.to_string_lossy() {
            return HealthStatus::error(
                "BAT_CONFIG_PATH is not set correctly",
                Some(format!(
                    "Add 'export BAT_CONFIG_PATH=\"{}\"' to your shell config",
                    expected_env.display()
                )),
            );
        }

        if !theme.is_empty() {
            let link = self.link_path(paths, theme);
            let theme_status = HealthStatus::check_file(&link, "Theme file");
            if theme_status.is_error() {
                return HealthStatus::error(
                    format!("Theme file `{theme}.tmTheme` is missing in bat themes directory"),
                    Some("Run `iris sync` or `iris health --fix` to relink and rebuild cache"),
                );
            }
        }

        HealthStatus::Ok
    }

    fn fix(
        &self,
        status: &HealthStatus,
        p: &Palette,
        paths: &IrisPaths,
        templater: &Templater,
        task: &mut Task,
    ) -> Result<()> {
        if !status.is_error() && !status.is_warning() {
            return task.log.action(
                &format!("Re-applied `{}` configuration", self.name().bold()),
                || {
                    self.apply(p, paths, templater, &mut task.as_quiet())?;
                    self.rebuild_bat_cache(&mut task.as_quiet())
                },
            );
        }

        let mut fixed = false;
        if status.contains("missing") || status.contains("not linked") {
            task.log.action(
                &format!("Repaired `{}` theme and configuration", self.name().bold()),
                || {
                    self.ensure_theme_cache(p, paths, templater)?;
                    self.ensure_config(p, paths)?;
                    let cache = self.cache_path(paths, &p.name);
                    let link = self.link_path(paths, &p.name);
                    self.ensure_symlink(&cache, &link)
                },
            )?;
            fixed = true;
        }

        if !fixed {
            task.log
                .action("Regenerated complete `bat` configuration", || {
                    self.apply(p, paths, templater, &mut task.as_quiet())
                })?;
        }

        self.rebuild_bat_cache(&mut task.as_quiet())?;
        Ok(())
    }

    fn remove_theme(&self, paths: &IrisPaths, theme_name: &str) -> Result<()> {
        let theme_lower: String = theme_name.to_lowercase();

        let cache_file: PathBuf = self.cache_path(paths, &theme_lower);
        if cache_file.exists() {
            fs::remove_file(cache_file)?;
        }

        let link_file: PathBuf = self.theme_path(paths, &theme_lower);
        if link_file.exists() {
            fs::remove_file(link_file)?;
        }

        self.rebuild_bat_cache(&mut Reporter::quiet().as_task())?;
        Ok(())
    }
}

impl BatGenerator {
    fn ensure_theme_cache(
        &self,
        p: &Palette,
        paths: &IrisPaths,
        templater: &Templater,
    ) -> Result<PathBuf> {
        let cache_path: PathBuf = self.cache_path(paths, &p.name);
        let render_ctx = self.build_render_context(p);
        let content: String = templater.render(&self.template_path(), &render_ctx)?;

        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "Failed to create `bat` theme directory: {}",
                    parent.display()
                )
            })?;
        }

        fs::write(&cache_path, content.trim()).with_context(|| {
            format!("Failed to write `bat` theme file: {}", cache_path.display())
        })?;
        Ok(cache_path)
    }

    fn ensure_symlink(&self, target: &Path, link: &Path) -> anyhow::Result<()> {
        if link.exists() || link.is_symlink() {
            fs::remove_file(link)
                .with_context(|| format!("Failed to remove old `bat` link: {}", link.display()))?;
        }

        if let Some(parent) = link.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "Failed to create directory for `bat` config: {}",
                    parent.display()
                )
            })?;
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            symlink(target, link).with_context(|| {
                format!(
                    "Failed to create `bat` symlink: {} -> {}",
                    target.display(),
                    link.display()
                )
            })?;
        }
        Ok(())
    }

    fn ensure_config(&self, p: &Palette, paths: &IrisPaths) -> anyhow::Result<()> {
        let theme_name: String = utils::capitalize(&p.name);
        let config_content: String = format!(
            "--theme=\"{name}\"\n--style=\"numbers,changes\"\n--color=\"always\"\n",
            name = theme_name
        );

        let generator_dir: PathBuf = paths.generators.join(self.name());
        let config_path: PathBuf = generator_dir.join("bat.conf");

        fs::create_dir_all(&generator_dir).with_context(|| {
            format!(
                "Failed to create `bat` generator directory: {}",
                generator_dir.display()
            )
        })?;

        fs::write(&config_path, config_content)
            .with_context(|| format!("Failed to write bat config: {}", config_path.display()))?;
        Ok(())
    }

    fn rebuild_bat_cache(&self, task: &mut Task) -> Result<()> {
        let task = task.log.step("Rebuilding `bat` cache...", false);
        let output = Command::new("bat")
            .arg("cache")
            .arg("--build")
            .output()
            .context("Failed to execute `bat` command. Is it installed and in your PATH?")?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("`bat` cache rebuild failed: {}", err.trim());
        }

        task.done_with("Cache rebuilt!");
        Ok(())
    }
}

/// Unit-tests for bat generator
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::tests::create_test_context;
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
    fn should_return_health_ok_for_bat() {
        let (_, mut ctx) = create_test_context();
        let generator = BatGenerator;
        let p = Palette::mock();

        let mut task = ctx.log.step("Test", false).as_quiet();
        ctx.state.current_theme = p.name.clone();
        generator
            .apply(&p, &ctx.paths, &ctx.templater, &mut task)
            .unwrap();
        let expected_config = ctx.paths.generators.join(generator.name()).join("bat.conf");

        temp_env::with_var("BAT_CONFIG_PATH", Some(expected_config), || {
            let status = generator.health_check(&ctx.paths, &p.name);
            assert!(status.is_ok(), "Expected Ok, got: {status}");
        });
    }

    #[test]
    fn should_return_health_error_bad_env_for_bat() {
        let (_tmp_dir, ctx) = create_test_context();
        let generator = BatGenerator;

        temp_env::with_var("BAT_CONFIG_PATH", Some("/wrong/path/to/bat/config"), || {
            let status = generator.health_check(&ctx.paths, &ctx.state.current_theme);
            assert!(
                status.is_error(),
                "Expected Error due to invalid env, got: {status}"
            );
            assert!(status.contains("BAT_CONFIG_PATH"));
        });
    }

    #[test]
    fn should_apply_theme_for_bat() {
        let (_, ctx) = create_test_context();
        let generator = BatGenerator;
        let p = Palette::mock();

        let expected_file_name: String = generator.target_file_name(&p.name);
        let expected_theme_name: String = utils::capitalize(&p.name);

        let mut task = ctx.log.step("Test", false).as_quiet();
        let result = generator.apply(&p, &ctx.paths, &ctx.templater, &mut task);
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
        let base_tmp: TempDir = TempDir::new("missing_test").unwrap();
        let home_dir = base_tmp.path();

        temp_env::with_vars([("HOME", Some(home_dir))], || {
            let (_iris_dir, ctx) = create_test_context();
            let generator = BatGenerator;
            let p = Palette::mock();
            let expected_env = ctx.paths.generators.join(generator.name()).join("bat.conf");

            temp_env::with_var("BAT_CONFIG_PATH", Some(expected_env.as_os_str()), || {
                let mut task = ctx.log.step("Test", false).as_quiet();
                generator
                    .apply(&p, &ctx.paths, &ctx.templater, &mut task)
                    .unwrap();

                let link = generator.link_path(&ctx.paths, &p.name);
                if link.exists() {
                    fs::remove_file(&link).unwrap();
                }

                let status = generator.health_check(&ctx.paths, &p.name);
                assert!(status.is_error(), "Expected Error, got: {status}");
                assert!(status.contains("missing"));

                generator
                    .fix(&status, &p, &ctx.paths, &ctx.templater, &mut task)
                    .unwrap();
                assert!(generator.health_check(&ctx.paths, &p.name).is_ok());
            });
        });
    }

    #[test]
    fn should_clear_generated_files_for_bat() {
        let (_, ctx) = create_test_context();
        let generator = BatGenerator;
        let p = Palette::mock();

        let mut task = ctx.log.step("Test", false).as_quiet();
        generator
            .apply(&p, &ctx.paths, &ctx.templater, &mut task)
            .unwrap();

        let cache_dir: PathBuf = ctx.paths.generators.join(generator.name());
        assert!(
            cache_dir.exists(),
            "Cache directory should exist before clearing"
        );

        generator.clear(&ctx.paths).unwrap();

        assert!(
            !cache_dir.exists(),
            "Clear should remove the entire generator cache directory"
        );
    }

    #[test]
    fn should_remove_theme_for_bat() {
        let (_, ctx) = create_test_context();
        let generator = BatGenerator;
        let p = Palette::mock();

        let mut task = ctx.log.step("Test", false).as_quiet();
        generator
            .apply(&p, &ctx.paths, &ctx.templater, &mut task)
            .unwrap();

        let cache_file = generator.cache_path(&ctx.paths, &p.name);
        let link_file = generator.theme_path(&ctx.paths, &p.name);

        assert!(
            cache_file.exists(),
            "Cache file should exist before removal"
        );
        assert!(link_file.exists(), "Theme file should exist before removal");

        generator.remove_theme(&ctx.paths, &p.name).unwrap();

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
