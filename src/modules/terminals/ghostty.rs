use crate::{
    commands::HealthStatus,
    core::IrisContext,
    models::Palette,
    modules::{Generator, GeneratorType},
    utils::{self},
};
use anyhow::{Context, Result};
use colored::Colorize;
use std::{fs, path::PathBuf};

/// Config generator for ghostty terminal
pub struct GhosttyGenerator;

impl Generator for GhosttyGenerator {
    fn name(&self) -> &str {
        "ghostty"
    }

    fn generator_type(&self) -> GeneratorType {
        GeneratorType::Terminal
    }

    fn target_file_name(&self, _theme: &str) -> String {
        "current_theme.conf".into()
    }

    fn cache_path(&self, ctx: &IrisContext, _theme_name: &str) -> PathBuf {
        ctx.paths
            .cache
            .join("ghostty")
            .join(self.target_file_name(""))
    }

    fn link_path(&self, _theme_name: &str) -> PathBuf {
        self.resolve_config_directory()
            .join(self.target_file_name(""))
    }

    fn apply(&self, p: &Palette, ctx: &IrisContext) -> Result<()> {
        let theme_name: &String = &p.name;
        let cache_file: PathBuf = self.cache_path(ctx, theme_name);
        let ghostty_dir: PathBuf = self.resolve_config_directory();
        let link_path: PathBuf = self.link_path(theme_name);

        let render_ctx = self.build_render_context(p);
        let content: String = ctx.templater.render(&self.template_path(), &render_ctx)?;

        fs::create_dir_all(cache_file.parent().unwrap())?;
        if !ghostty_dir.exists() {
            fs::create_dir_all(&ghostty_dir)?;
        }

        fs::write(&cache_file, content)?;

        if link_path.exists() || link_path.is_symlink() {
            fs::remove_file(&link_path)?;
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            ctx.log.info(&format!(
                "Linking {} theme to {}...",
                self.name().bold(),
                utils::pretty_path(&ghostty_dir).cyan()
            ));
            symlink(&cache_file, &link_path)
                .with_context(|| format!("Failed to link {:?} -> {:?}", link_path, cache_file))?;
        }

        Ok(())
    }

    fn build_render_context(&self, p: &Palette) -> tera::Context {
        let mut c = tera::Context::new();
        c.insert("theme_name", &utils::capitalize(&p.name));
        c.insert("bg", &p.bg);
        c.insert("fg", &p.fg);
        c.insert("sel_bg", &p.sel);
        c.insert("sel_fg", &p.bg);
        c.insert("cursor", &p.white);
        c.insert("ansi", &p.ansi);
        c
    }

    fn health_check(&self, ctx: &IrisContext) -> HealthStatus {
        if !self.is_installed() {
            return HealthStatus::Warning("Ghostty binary not found".into());
        }

        let ghostty_dir: PathBuf = self.resolve_config_directory();
        let config_path: PathBuf = ghostty_dir.join("config");
        let link_path: PathBuf = self.link_path("");
        let expected_cache: PathBuf = self.cache_path(ctx, "");

        if !link_path.exists() {
            return HealthStatus::Error {
                message: "current_theme.conf missing".into(),
                fix_hint: Some("run `iris sync` to create the link".into()),
            };
        }

        if !config_path.exists() {
            return HealthStatus::Error {
                message: "Ghostty config file missing".into(),
                fix_hint: Some(format!("Create {}", config_path.display())),
            };
        }

        let content = fs::read_to_string(&config_path).unwrap_or_default();
        let import_line = format!("config-file = {}", self.target_file_name(""));

        if !content.contains(&import_line) {
            return HealthStatus::Warning(format!(
                "Theme is generated but not imported in {}",
                config_path.display()
            ));
        }

        #[cfg(unix)]
        if let Ok(target) = std::fs::read_link(&link_path) {
            if target != expected_cache {
                return HealthStatus::Warning("Link points to a different cache location".into());
            }
        }

        HealthStatus::Ok
    }
}

/// Unit-tests for ghostty
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::create_test_context;

    #[test]
    fn should_return_ghostty_metadata() {
        let generator = GhosttyGenerator;
        assert_eq!(generator.name(), "ghostty");
        assert_eq!(generator.generator_type(), GeneratorType::Terminal);
        assert_eq!(generator.target_file_name("any"), "current_theme.conf");
    }

    #[test]
    fn should_build_valid_render_context() {
        let generator = GhosttyGenerator;
        let p = Palette::mock();
        let ctx = generator.build_render_context(&p);

        assert_eq!(ctx.get("bg").unwrap().as_str().unwrap(), p.bg);
        assert!(ctx.get("ansi").unwrap().is_array());
    }

    #[test]
    fn ghostty_health_ok() {
        let (tmp_dir, mut ctx) = create_test_context();
        let generator = GhosttyGenerator;
        let p = Palette::mock();
        let root = tmp_dir.path();

        temp_env::with_vars(
            vec![
                ("HOME", Some(root.to_str().unwrap())),
                (
                    "XDG_CONFIG_HOME",
                    Some(root.join(".config").to_str().unwrap()),
                ),
            ],
            || {
                ctx.state.current_theme = p.name.clone();
                generator.apply(&p, &ctx).unwrap();

                let config_path = generator.resolve_config_directory().join("config");
                let import_line = format!("config-file = {}", generator.target_file_name(&p.name));
                fs::write(&config_path, import_line).unwrap();

                let status = generator.health_check(&ctx);
                assert!(
                    matches!(status, HealthStatus::Ok),
                    "Expected Ok, got {:?}",
                    status
                );
            },
        );
    }

    #[test]
    fn ghostty_health_error_missing_config() {
        let (tmp_dir, ctx) = create_test_context();
        let generator = GhosttyGenerator;
        let root = tmp_dir.path();

        temp_env::with_vars(vec![("HOME", Some(root.to_str().unwrap()))], || {
            let theme_link = generator.link_path("test-theme");
            fs::create_dir_all(theme_link.parent().unwrap()).unwrap();
            fs::write(&theme_link, "").unwrap();

            let status = generator.health_check(&ctx);
            match status {
                HealthStatus::Error { ref message, .. } => {
                    assert!(message.contains("config file missing"));
                }
                _ => panic!("Expected Error for missing config, got {:?}", status),
            }
        });
    }

    #[test]
    fn ghostty_health_warning_no_import() {
        let (tmp_dir, mut ctx) = create_test_context();
        let generator = GhosttyGenerator;
        let p = Palette::mock();
        let root = tmp_dir.path();

        temp_env::with_vars(vec![("HOME", Some(root.to_str().unwrap()))], || {
            ctx.state.current_theme = p.name.clone();
            generator.apply(&p, &ctx).unwrap();

            let config_path = generator.resolve_config_directory().join("config");
            fs::write(&config_path, "font-family = JetBrainsMono").unwrap();

            let status = generator.health_check(&ctx);
            match status {
                HealthStatus::Warning(msg) => {
                    assert!(msg.contains("not imported"));
                }
                _ => panic!("Expected Warning for missing import line, got {:?}", status),
            }
        });
    }

    #[test]
    fn should_apply_theme_for_ghostty() {
        if which::which("ghostty").is_err() {
            return;
        }

        let (tmp_dir, ctx) = create_test_context();
        let generator = GhosttyGenerator;
        let p = Palette::mock();

        temp_env::with_vars(
            vec![
                ("XDG_CONFIG_HOME", Some(tmp_dir.path())),
                ("HOME", Some(tmp_dir.path())),
            ],
            || {
                let result = generator.apply(&p, &ctx);
                assert!(result.is_ok(), "Failed to apply: {:?}", result.err());

                let cache_file = ctx.paths.cache.join("ghostty").join("current_theme.conf");
                assert!(cache_file.exists());

                let content = fs::read_to_string(cache_file).unwrap();
                assert!(content.contains("background ="));
                assert!(content.contains("palette = 0="));
                assert!(content.contains("palette = 15="));
            },
        );
    }
}
