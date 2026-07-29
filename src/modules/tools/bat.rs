use super::rules::RULES;
use crate::{
    infra::IrisPaths,
    models::{HealthStatus, Issue, Theme},
    modules::{Cleanable, Generator, GeneratorType, Strategy, strategy::PipelineStep},
};
use std::{env, fs, path::PathBuf, process::Command};

/// Config generator for bat
pub struct BatGenerator;

impl Generator for BatGenerator {
    fn name(&self) -> &str {
        "bat"
    }

    fn generator_type(&self) -> GeneratorType {
        GeneratorType::Tool
    }

    fn strategy(&self) -> Strategy {
        Strategy::Pipeline { steps: vec![] }
    }

    fn pipeline_steps(&self, paths: &IrisPaths, theme: &Theme) -> Vec<PipelineStep> {
        let theme_lower: String = theme.name.to_lowercase();
        let cache_file: PathBuf = self.cache_path(paths, &theme_lower);
        let link_file: PathBuf = self.theme_path(paths, &theme_lower);
        let bat_conf_path: PathBuf = paths.bin.join("bat.conf");
        let zshrc_path: PathBuf = self.zshrc_path(paths);

        if let Some(parent) = link_file.parent() {
            let _ = fs::create_dir_all(parent);
        }

        vec![
            PipelineStep::GenerateFile {
                template_name: self.name().into(),
                destination: cache_file.clone(),
            },
            PipelineStep::RunCommand {
                program: "ln".into(),
                args: vec![
                    "-sf".into(),
                    cache_file.to_string_lossy().into(),
                    link_file.to_string_lossy().into(),
                ],
                silent: false,
            },
            PipelineStep::InjectBlock {
                file_path: bat_conf_path.clone(),
                marker: "batconf".into(),
                content: format!(
                    "--theme=\"{0}\"\n--style=\"numbers,changes\"\n--color=\"always\"",
                    theme_lower
                ),
            },
            PipelineStep::InjectBlock {
                file_path: zshrc_path,
                marker: "bat".into(),
                content: format!(
                    "export BAT_CONFIG_PATH=\"{}\"",
                    crate::utils::pretty_path(&bat_conf_path)
                ),
            },
            PipelineStep::RunCommand {
                program: "bat".into(),
                args: vec!["cache".into(), "--build".into()],
                silent: true,
            },
        ]
    }

    fn target_file_name(&self, theme: &str) -> String {
        format!("{}.tmTheme", theme)
    }

    fn enrich_context(&self, context: &mut tera::Context, theme: &Theme) -> anyhow::Result<()> {
        let processed_rules: Vec<serde_json::Value> = RULES
            .iter()
            .map(|r| {
                let color: &str = r.color_key.resolve(&theme.colors);
                let style = if r.style.is_empty() || r.style == "normal" {
                    None
                } else {
                    Some(r.style)
                };

                serde_json::json!({
                    "name": r.name,
                    "scope": r.scope,
                    "style": style,
                    "foreground": color,
                })
            })
            .collect();

        context.insert("rules", &processed_rules);
        Ok(())
    }

    fn health_check(&self, paths: &IrisPaths, theme: &str) -> HealthStatus {
        if !self.is_installed() {
            return HealthStatus::error(Issue::BinaryNotFound);
        }

        if !theme.is_empty() {
            let link: PathBuf = self.theme_path(paths, theme);
            let theme_status = HealthStatus::check_file(&link, Issue::CacheMissing);
            if !theme_status.is_ok() {
                return theme_status;
            }
        }

        let zshrc: PathBuf = self.zshrc_path(paths);
        if zshrc.exists() {
            let zshrc_content = fs::read_to_string(&zshrc).unwrap_or_default();
            if !zshrc_content.contains("BAT_CONFIG_PATH") {
                return HealthStatus::warn(Issue::ConfigMissing);
            }
        }

        let expected_env: PathBuf = paths.bin.join("bat.conf");
        let current_env: String = env::var("BAT_CONFIG_PATH").unwrap_or_default();

        if current_env != expected_env.to_string_lossy() {
            return HealthStatus::warn(Issue::EnvMismatch);
        }

        HealthStatus::Ok
    }

    fn as_cleanable(&self) -> Option<&dyn Cleanable> {
        Some(self)
    }
}

impl BatGenerator {
    fn theme_path(&self, paths: &IrisPaths, theme_name: &str) -> PathBuf {
        self.resolve_config_directory(paths)
            .join("themes")
            .join(self.target_file_name(theme_name))
    }

    fn zshrc_path(&self, paths: &IrisPaths) -> PathBuf {
        paths
            .config
            .parent()
            .and_then(|p| p.parent())
            .unwrap_or(&paths.config)
            .join(".zshrc")
    }

    fn rebuild_bat_cache(&self) -> anyhow::Result<()> {
        let output = Command::new("bat").arg("cache").arg("--build").output()?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("`bat` cache rebuild failed: {}", err.trim());
        }

        Ok(())
    }
}

impl Cleanable for BatGenerator {
    fn cleanup(&self, paths: &IrisPaths) -> anyhow::Result<()> {
        crate::modules::cleanable::default_cleanup(self, paths)
    }

    fn remove_theme(&self, paths: &IrisPaths, theme_name: &str) -> anyhow::Result<()> {
        crate::modules::cleanable::default_remove(self, paths, theme_name)
    }

    fn post_cleanup(&self, paths: &IrisPaths) -> anyhow::Result<()> {
        let bat_conf_path: PathBuf = paths.bin.join("bat.conf");
        if bat_conf_path.exists() {
            fs::remove_file(&bat_conf_path)?;
        }
        Ok(())
    }

    fn post_remove(&self, _paths: &IrisPaths, _theme_name: &str) -> anyhow::Result<()> {
        self.rebuild_bat_cache()?;
        Ok(())
    }
}

/// Tests for bat generator
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::IrisContext;
    use temp_env;
    use tempdir::TempDir;

    const MOCK_TEMPLATE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0">
<dict>
    <key>name</key>
    <string>{{ theme_name }}</string>
    <key>settings</key>
    <array>
        <dict>
            <key>settings</key>
            <dict>
                <key>background</key><string>{{ bg }}</string>
                <key>foreground</key><string>{{ fg }}</string>
            </dict>
        </dict>
    </array>
</dict>
</plist>"#;

    /// Unit-tests for bat
    mod unit {
        use super::*;

        #[test]
        fn should_return_metadata_for_bat() {
            let generator = BatGenerator;
            assert_eq!(generator.name(), "bat");
            assert_eq!(generator.generator_type(), GeneratorType::Tool);
            assert_eq!(generator.target_file_name("nord"), "nord.tmTheme");
        }

        #[test]
        fn should_build_valid_render_context_for_bat() {
            let generator = BatGenerator;
            let theme: Theme = Theme::mock();
            let (_, ctx) = IrisContext::with_templates(vec![("tools/bat", MOCK_TEMPLATE)]);
            let ctx = ctx.engine(&theme).build_context(&generator).unwrap();

            assert_eq!(ctx.get("theme_name").unwrap().as_str().unwrap(), theme.name);
            assert!(ctx.contains_key("bg"));
            assert!(ctx.contains_key("fg"));
        }

        #[test]
        fn should_apply_theme_for_bat() {
            let (_, ctx) = IrisContext::with_templates(vec![("tools/bat", MOCK_TEMPLATE)]);
            let generator = BatGenerator;
            let theme: Theme = Theme::new("Test-Theme", Theme::mock().colors);
            let test_bat_cache = ctx.paths.generators.join("bat").join("cache");

            temp_env::with_var("BAT_CACHE_PATH", Some(&test_bat_cache), || {
                let expected_file_name: String = generator.target_file_name(&theme.name);
                let expected_theme_name: String = theme.name.to_lowercase();

                let mut activity = ctx.log.step("Test", false).muted();
                let result = ctx.engine(&theme).execute_apply(&generator, &mut activity);
                assert!(result.is_ok(), "Apply failed: {:?}", result.err());

                let cache_theme_path = ctx.paths.generators.join("bat").join(&expected_file_name);
                let bat_conf_path: PathBuf = ctx.paths.bin.join("bat.conf");

                assert!(cache_theme_path.exists(), "Theme file missing in cache");
                assert!(bat_conf_path.exists(), "bat.conf missing in bin/");

                let conf_content = fs::read_to_string(bat_conf_path).unwrap();
                assert!(conf_content.contains(&format!("--theme=\"{}\"", expected_theme_name)));
            });
        }

        #[test]
        fn should_clear_generated_files_for_bat() {
            let (_, ctx) = IrisContext::mock();
            let generator = BatGenerator;
            let theme: Theme = Theme::mock();
            let test_bat_cache = ctx.paths.generators.join("bat").join("cache");

            temp_env::with_var("BAT_CACHE_PATH", Some(&test_bat_cache), || {
                let mut activity = ctx.log.step("Test", false).muted();
                ctx.engine(&theme)
                    .execute_apply(&generator, &mut activity)
                    .unwrap();

                let cache_dir: PathBuf = ctx.paths.generators.join(generator.name());
                let bat_conf_path: PathBuf = ctx.paths.bin.join("bat.conf");

                assert!(cache_dir.exists());
                assert!(bat_conf_path.exists());

                generator.cleanup(&ctx.paths).unwrap();

                assert!(!cache_dir.exists());
                assert!(!bat_conf_path.exists());
            });
        }

        #[test]
        fn should_remove_theme_for_bat() {
            let (_, ctx) = IrisContext::mock();
            let generator = BatGenerator;
            let theme: Theme = Theme::mock();
            let test_bat_cache = ctx.paths.generators.join("bat").join("cache");

            temp_env::with_var("BAT_CACHE_PATH", Some(&test_bat_cache), || {
                let mut activity = ctx.log.step("Test", false).muted();
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
            });
        }
    }

    /// Integration tests for bat
    mod integration {
        use super::*;
        use crate::skip_if_not_installed;

        #[test]
        fn should_return_health_ok_for_bat() {
            skip_if_not_installed!(BatGenerator);

            let (_, mut ctx) = IrisContext::with_templates(vec![("tools/bat", MOCK_TEMPLATE)]);
            let generator = BatGenerator;
            let theme: Theme = Theme::mock();
            let test_bat_cache = ctx.paths.generators.join("bat").join("cache");

            temp_env::with_var("BAT_CACHE_PATH", Some(&test_bat_cache), || {
                let mut activity = ctx.log.step("Test", false).muted();
                ctx.state.theme.current_theme = theme.name.clone();

                ctx.engine(&theme)
                    .execute_apply(&generator, &mut activity)
                    .unwrap();

                let expected_config = ctx.paths.bin.join("bat.conf");
                temp_env::with_var("BAT_CONFIG_PATH", Some(expected_config), || {
                    let status = generator.health_check(&ctx.paths, &theme.name);
                    assert!(status.is_ok(), "Expected Ok, got: {status}");
                });
            });
        }

        #[test]
        fn should_return_health_error_env_mismatch_for_bat() {
            skip_if_not_installed!(BatGenerator);

            let (_tmp_dir, ctx) = IrisContext::mock();
            let generator = BatGenerator;

            temp_env::with_var("BAT_CONFIG_PATH", Some("/wrong/path/to/bat/config"), || {
                let status = generator.health_check(&ctx.paths, &ctx.state.theme.current_theme);
                assert!(status.is_warning(), "Expected Warning, got: {status}");
                assert!(status.contains("Environment variable mismatch"));
            });
        }

        #[test]
        fn should_return_health_error_missing_zshrc_import_for_bat() {
            skip_if_not_installed!(BatGenerator);

            let (_, ctx) = IrisContext::mock();
            let generator = BatGenerator;
            let zshrc = generator.zshrc_path(&ctx.paths);

            if let Some(parent) = zshrc.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&zshrc, "# empty zshrc").unwrap();

            let expected_env = ctx.paths.bin.join("bat.conf");
            temp_env::with_var("BAT_CONFIG_PATH", Some(expected_env.as_os_str()), || {
                let status = generator.health_check(&ctx.paths, "");
                assert!(status.is_warning());
                assert!(status.contains("Configuration file missing"));
            });
        }

        #[test]
        fn should_fix_missing_link_for_bat() {
            skip_if_not_installed!(BatGenerator);

            let base_tmp: TempDir = TempDir::new("missing_test").unwrap();
            let home_dir = base_tmp.path();
            let (_iris_dir, ctx) = IrisContext::mock();
            let generator = BatGenerator;
            let theme: Theme = Theme::mock();
            let test_bat_cache = ctx.paths.generators.join("bat").join("cache");

            temp_env::with_vars(
                [
                    ("HOME", Some(home_dir.as_os_str())),
                    ("BAT_CACHE_PATH", Some(test_bat_cache.as_os_str())),
                ],
                || {
                    let expected_env = ctx.paths.bin.join("bat.conf");

                    temp_env::with_var("BAT_CONFIG_PATH", Some(expected_env.as_os_str()), || {
                        let mut activity = ctx.log.step("Test", false).muted();
                        let engine = ctx.engine(&theme);
                        engine.execute_apply(&generator, &mut activity).unwrap();

                        let link = generator.theme_path(&ctx.paths, &theme.name);
                        if link.exists() || link.is_symlink() {
                            fs::remove_file(&link).unwrap();
                        }

                        let status = generator.health_check(&ctx.paths, &theme.name);
                        assert!(
                            status.is_error(),
                            "Expected error for missing link, got: {status}"
                        );

                        generator.fix(&status, &engine, &mut activity).unwrap();
                        assert!(generator.health_check(&ctx.paths, &theme.name).is_ok());
                    });
                },
            );
        }
    }
}
