use super::rules::RULES;
use crate::{
    core::{IrisPaths, Templater},
    guards::FsRollbackGuard,
    log::{Activity, Logger},
    models::{HealthStatus, Theme},
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
        theme: &Theme,
        paths: &IrisPaths,
        templater: &Templater,
        task: &mut Activity,
    ) -> Result<()> {
        task.info(&format!(
            "Generating {} theme for {}...",
            theme.name.yellow(),
            self.name().bold().cyan()
        ));

        let cache_theme: PathBuf = self.ensure_theme_cache(theme, paths, templater)?;
        let link_path: PathBuf = self.link_path(paths, &theme.name.to_lowercase());
        let backup_path: PathBuf = link_path.with_extension("bak");

        task.info(&format!(
            "Linking {} theme to {}...",
            self.name().bold(),
            utils::pretty_path(&link_path).magenta(),
        ));

        let rollback_guard = FsRollbackGuard::new(link_path.clone(), backup_path);

        self.ensure_symlink(&cache_theme, &link_path)?;
        self.ensure_config(theme, paths)?;
        self.rebuild_bat_cache(task)?;
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
        let fix = |h: &str| -> String {
            let hex = h.trim_start_matches('#');
            format!("#{}", hex)
        };

        c.insert("theme_name", &theme.name);
        c.insert("bg", &fix(&theme.colors.bg));
        c.insert("fg", &fix(&theme.colors.fg));
        c.insert("sel", &fix(&theme.colors.sel));
        c.insert("line", &fix(&theme.colors.line_hl));

        let processed_rules: Vec<serde_json::Value> = RULES
            .iter()
            .map(|r| {
                let color = match r.color_key {
                    "keyword" => &theme.colors.keyword,
                    "func" => &theme.colors.func,
                    "type_name" => &theme.colors.type_name,
                    "string" => &theme.colors.string,
                    "operator" => &theme.colors.operator,
                    "number" => &theme.colors.number,
                    "comment" => &theme.colors.comment,
                    _ => &theme.colors.fg,
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
        theme: &Theme,
        paths: &IrisPaths,
        templater: &Templater,
        task: &mut Activity,
    ) -> Result<()> {
        if !status.is_error() && !status.is_warning() {
            return task.log.action(
                &format!("Re-applied `{}` configuration", self.name().bold()),
                || {
                    self.apply(theme, paths, templater, &mut task.muted())?;
                    self.rebuild_bat_cache(&mut task.muted())
                },
            );
        }

        let mut fixed = false;
        if status.contains("missing") || status.contains("not linked") {
            let link: PathBuf = self.link_path(paths, &theme.name.to_lowercase());
            let backup: PathBuf = link.with_extension("bak");
            let cache: PathBuf = self.cache_path(paths, &theme.name.to_lowercase());

            let rollback_guard = FsRollbackGuard::new(link.clone(), backup);

            task.log.action(
                &format!("Repaired `{}` theme and configuration", self.name().bold()),
                || {
                    self.ensure_theme_cache(theme, paths, templater)?;
                    self.ensure_config(theme, paths)?;
                    self.ensure_symlink(&cache, &link)
                },
            )?;

            rollback_guard.commit();
            fixed = true;
        }

        if !fixed {
            task.log
                .action("Regenerated complete `bat` configuration", || {
                    self.apply(theme, paths, templater, &mut task.muted())
                })?;
        }

        self.rebuild_bat_cache(&mut task.muted())?;
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

        self.rebuild_bat_cache(&mut Logger::silent().as_task())?;
        Ok(())
    }
}

impl BatGenerator {
    fn ensure_theme_cache(
        &self,
        theme: &Theme,
        paths: &IrisPaths,
        templater: &Templater,
    ) -> Result<PathBuf> {
        let cache_path: PathBuf = self.cache_path(paths, &theme.name.to_lowercase());
        let render_ctx = self.build_render_context(theme);
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

    fn ensure_config(&self, theme: &Theme, paths: &IrisPaths) -> anyhow::Result<()> {
        let config_content: String = format!(
            "--theme=\"{name}\"\n--style=\"numbers,changes\"\n--color=\"always\"\n",
            name = theme.name
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

    fn rebuild_bat_cache(&self, task: &mut Activity) -> Result<()> {
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
        let theme: Theme = Theme::mock();
        let context = generator.build_render_context(&theme);
        let data = context.into_json();

        assert_eq!(data["theme_name"], theme.name);
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
        let theme: Theme = Theme::mock();
        let test_bat_cache = ctx.paths.generators.join("bat").join("cache");

        temp_env::with_var("BAT_CACHE_PATH", Some(&test_bat_cache), || {
            let mut task = ctx.log.step("Test", false).muted();
            ctx.state.current_theme = theme.name.clone();

            generator
                .apply(&theme, &ctx.paths, &ctx.templater, &mut task)
                .unwrap();

            let expected_config = ctx.paths.generators.join(generator.name()).join("bat.conf");

            temp_env::with_var("BAT_CONFIG_PATH", Some(expected_config), || {
                let status = generator.health_check(&ctx.paths, &theme.name);
                assert!(status.is_ok(), "Expected Ok, got: {status}");
            });
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
        let theme: Theme = Theme::new("Test-Theme", Theme::mock().colors);
        let test_bat_cache = ctx.paths.generators.join("bat").join("cache");

        temp_env::with_var("BAT_CACHE_PATH", Some(&test_bat_cache), || {
            let expected_file_name: String = generator.target_file_name(&theme.name);
            let expected_theme_name: String = theme.name.clone();

            let mut task = ctx.log.step("Test", false).muted();
            let result = generator.apply(&theme, &ctx.paths, &ctx.templater, &mut task);
            assert!(result.is_ok(), "Apply failed: {:?}", result.err());

            let cache_theme_path: PathBuf =
                ctx.paths.generators.join("bat").join(&expected_file_name);
            let bat_conf_path: PathBuf = ctx.paths.generators.join("bat").join("bat.conf");

            assert!(cache_theme_path.exists());
            assert!(bat_conf_path.exists());

            let conf_content = fs::read_to_string(bat_conf_path).unwrap();
            assert!(conf_content.contains(&format!("--theme=\"{}\"", expected_theme_name)));
        });
    }

    #[test]
    fn should_fix_missing_link_for_bat() {
        let base_tmp: TempDir = TempDir::new("missing_test").unwrap();
        let home_dir = base_tmp.path();
        let (_iris_dir, ctx) = create_test_context();
        let generator = BatGenerator;
        let theme: Theme = Theme::mock();
        let test_bat_cache = ctx.paths.generators.join("bat").join("cache");

        temp_env::with_vars(
            [
                ("HOME", Some(home_dir.as_os_str())),
                ("BAT_CACHE_PATH", Some(test_bat_cache.as_os_str())),
            ],
            || {
                let expected_env = ctx.paths.generators.join(generator.name()).join("bat.conf");

                temp_env::with_var("BAT_CONFIG_PATH", Some(expected_env.as_os_str()), || {
                    let mut task = ctx.log.step("Test", false).muted();
                    generator
                        .apply(&theme, &ctx.paths, &ctx.templater, &mut task)
                        .unwrap();

                    let link = generator.link_path(&ctx.paths, &theme.name);
                    if link.exists() {
                        fs::remove_file(&link).unwrap();
                    }

                    let status = generator.health_check(&ctx.paths, &theme.name);
                    assert!(status.is_error());
                    assert!(status.contains("missing"));

                    generator
                        .fix(&status, &theme, &ctx.paths, &ctx.templater, &mut task)
                        .unwrap();
                    assert!(generator.health_check(&ctx.paths, &theme.name).is_ok());
                });
            },
        );
    }

    #[test]
    fn should_clear_generated_files_for_bat() {
        let (_, ctx) = create_test_context();
        let generator = BatGenerator;
        let theme: Theme = Theme::mock();
        let test_bat_cache = ctx.paths.generators.join("bat").join("cache");

        temp_env::with_var("BAT_CACHE_PATH", Some(&test_bat_cache), || {
            let mut task = ctx.log.step("Test", false).muted();
            generator
                .apply(&theme, &ctx.paths, &ctx.templater, &mut task)
                .unwrap();

            let cache_dir: PathBuf = ctx.paths.generators.join(generator.name());
            assert!(cache_dir.exists());

            generator.clear(&ctx.paths).unwrap();
            assert!(!cache_dir.exists());
        });
    }

    #[test]
    fn should_remove_theme_for_bat() {
        let (_, ctx) = create_test_context();
        let generator = BatGenerator;
        let theme: Theme = Theme::mock();
        let test_bat_cache = ctx.paths.generators.join("bat").join("cache");

        temp_env::with_var("BAT_CACHE_PATH", Some(&test_bat_cache), || {
            let mut task = ctx.log.step("Test", false).muted();
            generator
                .apply(&theme, &ctx.paths, &ctx.templater, &mut task)
                .unwrap();

            let cache_file = generator.cache_path(&ctx.paths, &theme.name);
            let link_file = generator.theme_path(&ctx.paths, &theme.name);

            assert!(cache_file.exists());
            assert!(link_file.exists());

            generator.remove_theme(&ctx.paths, &theme.name).unwrap();

            assert!(!cache_file.exists());
            assert!(!link_file.exists());
        });
    }
}
