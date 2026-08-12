use super::rules::RULES;
use crate::{
    infra::IrisPaths,
    models::{HealthStatus, Issue, Theme},
    modules::{Generator, GeneratorType, Strategy, strategy::PipelineStep, traits::*},
};
use std::{env, fs, path::PathBuf, process::Command};

/// Config generator for bat
pub struct BatGenerator;

impl Identifiable for BatGenerator {
    fn name(&self) -> &'static str {
        "bat"
    }

    fn generator_type(&self) -> GeneratorType {
        GeneratorType::Tool
    }
}

impl PathResolvable for BatGenerator {
    fn base_file_name(&self) -> String {
        "config".into()
    }

    fn file_name(&self, theme: &str) -> String {
        format!("{}.tmTheme", theme)
    }

    fn link_path(&self, paths: &IrisPaths, theme: &str) -> PathBuf {
        self.theme_path(paths, theme)
    }
}

impl Generator for BatGenerator {
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
                    "export BAT_CONFIG_PATH={}",
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
}

impl BatGenerator {
    fn theme_path(&self, paths: &IrisPaths, theme_name: &str) -> PathBuf {
        self.config_dir(paths)
            .join("themes")
            .join(self.file_name(theme_name))
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

impl Diagnosable for BatGenerator {
    fn health_check(&self, paths: &IrisPaths, theme: &str) -> HealthStatus {
        if !self.is_installed() {
            return HealthStatus::error(Issue::BinaryNotFound);
        }

        if !theme.is_empty() {
            let link = self.theme_path(paths, theme);
            if !link.exists() {
                return HealthStatus::error(Issue::CacheMissing);
            }
        }

        let zshrc: PathBuf = self.zshrc_path(paths);
        if zshrc.exists() {
            if let Ok(content) = fs::read_to_string(&zshrc) {
                if !content.contains("BAT_CONFIG_PATH") {
                    return HealthStatus::warn(Issue::ConfigMissing);
                }
            }
        }

        let expected_env: PathBuf = paths.bin.join("bat.conf");
        let current_env: String = env::var("BAT_CONFIG_PATH").unwrap_or_default();

        if current_env != expected_env.to_string_lossy() {
            return HealthStatus::warn(Issue::EnvMismatch);
        }

        HealthStatus::Ok
    }
}

impl Cleanable for BatGenerator {
    fn cleanup(&self, paths: &IrisPaths) -> anyhow::Result<()> {
        default_cleanup(self, paths)
    }

    fn remove_theme(&self, paths: &IrisPaths, theme_name: &str) -> anyhow::Result<()> {
        default_remove(self, paths, theme_name)
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

impl Diffable for BatGenerator {
    fn diff_style(&self) -> DiffStyle {
        let gen_name = self.name().to_string();

        DiffStyle::Custom(Box::new(move |_, theme, config_path, paths| {
            if theme.is_empty() {
                return Ok(None);
            }

            let theme_lower: String = theme.to_lowercase();
            let bat_conf_path: PathBuf = paths.bin.join("bat.conf");
            let zshrc_path: PathBuf = paths
                .config
                .parent()
                .and_then(|p| p.parent())
                .unwrap_or(&paths.config)
                .join(".zshrc");

            let bat_inner: String = format!(
                "--theme=\"{}\"\n--style=\"numbers,changes\"\n--color=\"always\"",
                theme_lower
            );
            let bat_current: String = fs::read_to_string(&bat_conf_path).unwrap_or_default();
            let bat_block: String = format!(
                "# [iris:begin:batconf]\n{}\n# [iris:end:batconf]",
                bat_inner
            );
            let bat_ok = crate::utils::block_matches(&bat_current, "batconf", &bat_block);

            let zshrc_inner = format!(
                "export BAT_CONFIG_PATH={}",
                crate::utils::pretty_path(&bat_conf_path)
            );
            let zshrc_current = fs::read_to_string(&zshrc_path).unwrap_or_default();
            let zshrc_block = format!(
                "# [iris:begin:{}]\n{}\n# [iris:end:{}]",
                gen_name, zshrc_inner, gen_name
            );
            let zshrc_ok = zshrc_current.is_empty()
                || crate::utils::block_matches(&zshrc_current, &gen_name, &zshrc_block);

            if bat_ok && zshrc_ok {
                return Ok(None);
            }

            let final_bat = crate::utils::replace_block(&bat_current, "batconf", &bat_inner);
            if bat_current.trim() != final_bat.trim() {
                return diffable::render_diff(config_path, &bat_current, &final_bat);
            }

            if !zshrc_current.is_empty() && !zshrc_ok {
                let final_zshrc =
                    crate::utils::replace_block(&zshrc_current, &gen_name, &zshrc_inner);
                return diffable::render_diff(&zshrc_path, &zshrc_current, &final_zshrc);
            }

            Ok(None)
        }))
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
            assert_eq!(generator.file_name("nord"), "nord.tmTheme");
        }

        #[test]
        fn should_handle_path_resolution_for_bat() {
            let (_temp_dir, ctx) = IrisContext::mock();
            let generator = BatGenerator;
            let theme = "tokyonight";

            let expected_config_dir = generator.config_dir(&ctx.paths);
            assert_eq!(generator.config_dir(&ctx.paths), expected_config_dir);

            let expected_config_path = generator.config_path(&ctx.paths);
            assert_eq!(generator.config_path(&ctx.paths), expected_config_path);

            let expected_cache_path = ctx.paths.generators.join("bat/tokyonight.tmTheme");
            assert_eq!(generator.cache_path(&ctx.paths, theme), expected_cache_path);

            let expected_link_path = generator.theme_path(&ctx.paths, theme);
            assert!(
                expected_link_path
                    .to_string_lossy()
                    .contains("tokyonight.tmTheme")
            );

            assert_eq!(generator.template_path(), "tools/bat");
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
            let bat_conf_path: PathBuf = ctx.paths.bin.join("bat.conf");

            if let Some(parent) = bat_conf_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&bat_conf_path, "").unwrap();

            temp_env::with_var("BAT_CACHE_PATH", Some(&test_bat_cache), || {
                let expected_file_name: String = generator.file_name(&theme.name);
                let expected_theme_name: String = theme.name.to_lowercase();

                let mut activity = ctx.log.step("Test", false).muted();
                let result = ctx.engine(&theme).execute_apply(&generator, &mut activity);
                assert!(result.is_ok(), "Apply failed: {:?}", result.err());

                let cache_theme_path = ctx.paths.generators.join("bat").join(&expected_file_name);

                assert!(cache_theme_path.exists(), "Theme file missing in cache");
                assert!(bat_conf_path.exists(), "bat.conf missing in bin/");

                let conf_content = fs::read_to_string(&bat_conf_path).unwrap();
                assert!(conf_content.contains(&format!("--theme=\"{}\"", expected_theme_name)));
            });
        }

        #[test]
        fn should_clear_generated_files_for_bat() {
            let (_, ctx) = IrisContext::mock();
            let generator = BatGenerator;
            let theme: Theme = Theme::mock();
            let test_bat_cache = ctx.paths.generators.join("bat").join("cache");
            let bat_conf_path: PathBuf = ctx.paths.bin.join("bat.conf");

            if let Some(parent) = bat_conf_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&bat_conf_path, "").unwrap();

            temp_env::with_var("BAT_CACHE_PATH", Some(&test_bat_cache), || {
                let mut activity = ctx.log.step("Test", false).muted();
                ctx.engine(&theme)
                    .execute_apply(&generator, &mut activity)
                    .unwrap();

                let cache_dir: PathBuf = ctx.paths.generators.join(generator.name());

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
            let bat_conf_path: PathBuf = ctx.paths.bin.join("bat.conf");

            if let Some(parent) = bat_conf_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&bat_conf_path, "").unwrap();

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

            temp_env::with_var("BAT_CONFIG_PATH", Some("/wrong/path/config.conf"), || {
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
            let (_iris_dir, ctx) = IrisContext::with_templates(vec![("tools/bat", MOCK_TEMPLATE)]);
            let generator = BatGenerator;
            let theme: Theme = Theme::mock();
            let test_bat_cache = ctx.paths.generators.join("bat").join("cache");
            let expected_env = ctx.paths.bin.join("bat.conf");

            if let Some(parent) = expected_env.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&expected_env, "").unwrap();

            temp_env::with_vars(
                [
                    ("HOME", Some(home_dir.as_os_str())),
                    ("BAT_CACHE_PATH", Some(test_bat_cache.as_os_str())),
                ],
                || {
                    temp_env::with_var("BAT_CONFIG_PATH", Some(expected_env.as_os_str()), || {
                        let mut activity = ctx.log.step("Test", false).muted();
                        let engine = ctx.engine(&theme);
                        engine.execute_apply(&generator, &mut activity).unwrap();

                        let link = generator.theme_path(&ctx.paths, &theme.name);
                        if link.exists() || link.is_symlink() {
                            fs::remove_file(&link).unwrap();
                        }

                        let status = generator.health_check(&ctx.paths, &theme.name);
                        assert!(status.is_error(), "Expected Error, got: {status}");

                        engine
                            .execute_fix(&generator, &status, &mut activity)
                            .unwrap();
                        assert!(generator.health_check(&ctx.paths, &theme.name).is_ok());
                    });
                },
            );
        }
    }
}
