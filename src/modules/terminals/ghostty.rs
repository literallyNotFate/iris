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
    fs,
    path::{Path, PathBuf},
};

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
            .generators
            .join(self.name())
            .join(self.target_file_name(""))
    }

    fn link_path(&self, _theme_name: &str) -> PathBuf {
        self.resolve_config_directory()
            .join(self.target_file_name(""))
    }

    fn apply(&self, p: &Palette, ctx: &IrisContext) -> Result<()> {
        ctx.log.info(&format!(
            "Generating {} theme for {}",
            utils::capitalize(&p.name).yellow(),
            self.name().bold().cyan(),
        ));
        let cache_file: PathBuf = self.ensure_cache_file(p, ctx)?;
        let link_path: PathBuf = self.link_path(&p.name);

        ctx.log.info(&format!(
            "Linking {} theme to {}...",
            self.name().bold(),
            utils::pretty_path(&link_path).magenta(),
        ));

        self.ensure_symlink(&cache_file, &link_path, ctx)?;

        ctx.log.info(&format!(
            "{} theme applied to {}",
            utils::capitalize(&p.name).yellow(),
            self.name().bold().cyan()
        ));
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

    fn fix(&self, status: &HealthStatus, p: &Palette, ctx: &IrisContext) -> Result<()> {
        match status {
            HealthStatus::Error { message, .. } => {
                let msg_low: String = message.to_lowercase();
                if msg_low.contains("missing") || msg_low.contains("not found") {
                    ctx.log
                        .step(
                            &format!(
                                "Repairing {} configuration and paths...",
                                self.name().bold()
                            ),
                            2,
                        )
                        .done(true);

                    let cache = self.cache_path(ctx, &p.name);
                    let link = self.link_path(&p.name);
                    self.ensure_symlink(&cache, &link, &ctx.silent())?;
                }

                self.apply(p, &ctx.silent())
            }
            HealthStatus::Warning(msg) if msg.contains("not imported") => {
                ctx.log
                    .step(
                        &format!("Injecting import into {}...", self.name().bold()),
                        2,
                    )
                    .done(true);
                self.inject_import_line()
            }
            _ => self.apply(p, &ctx.silent()),
        }
    }
}

impl GhosttyGenerator {
    fn ensure_cache_file(&self, p: &Palette, ctx: &IrisContext) -> Result<PathBuf> {
        let cache_file = self.cache_path(ctx, &p.name);
        let render_ctx = self.build_render_context(p);
        let content = ctx.templater.render(&self.template_path(), &render_ctx)?;

        fs::create_dir_all(cache_file.parent().unwrap())?;
        fs::write(&cache_file, content)?;
        Ok(cache_file)
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
                .with_context(|| format!("Failed to link {:?} -> {:?}", target, link))?;
        }
        Ok(())
    }

    fn inject_import_line(&self) -> Result<()> {
        let config_path = self.resolve_config_directory().join("config");
        let import_line = format!("\nconfig-file = {}\n", self.target_file_name(""));

        if !config_path.exists() {
            fs::write(&config_path, import_line)?;
        } else {
            let mut content = fs::read_to_string(&config_path)?;
            if !content.contains(&import_line.trim()) {
                content.push_str(&import_line);
                fs::write(&config_path, content)?;
            }
        }
        Ok(())
    }
}

/// Unit-tests for ghostty
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::tests::create_test_context;

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

        temp_env::with_vars(
            vec![
                ("HOME", Some(root.to_str().unwrap())),
                ("XDG_CONFIG_HOME", None),
            ],
            || {
                let config_dir = generator.resolve_config_directory();
                let theme_link = generator.link_path("");

                assert!(
                    config_dir.starts_with(root),
                    "Config dir {:?} is outside of root {:?}",
                    config_dir,
                    root
                );

                fs::create_dir_all(theme_link.parent().unwrap()).unwrap();
                fs::create_dir_all(&config_dir).unwrap();
                fs::write(&theme_link, "palette = []").unwrap();

                let config_path = config_dir.join("config");
                if config_path.exists() {
                    fs::remove_file(&config_path).unwrap();
                }

                let status = generator.health_check(&ctx);
                match status {
                    HealthStatus::Error { ref message, .. } => {
                        assert!(
                            message.to_lowercase().contains("config"),
                            "Message was: {}",
                            message
                        );
                    }
                    _ => panic!("Expected Error for missing config, but got: {:?}", status),
                }
            },
        );
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

                let cache_file = ctx
                    .paths
                    .generators
                    .join("ghostty")
                    .join("current_theme.conf");
                assert!(cache_file.exists());

                let content = fs::read_to_string(cache_file).unwrap();
                assert!(content.contains("background ="));
                assert!(content.contains("palette = 0="));
                assert!(content.contains("palette = 15="));
            },
        );
    }

    #[test]
    fn should_fix_inject_issue_for_ghostty() {
        let (tmp_dir, ctx) = create_test_context();
        let generator = GhosttyGenerator;
        let p = Palette::mock();
        let root = tmp_dir.path();

        temp_env::with_var("HOME", Some(root), || {
            let config_dir = generator.resolve_config_directory();
            fs::create_dir_all(&config_dir).unwrap();
            let config_path = config_dir.join("config");

            generator.apply(&p, &ctx).unwrap();
            fs::write(&config_path, "font-size = 12").unwrap();

            let status = generator.health_check(&ctx);
            generator.fix(&status, &p, &ctx.silent()).unwrap();

            let content = fs::read_to_string(&config_path).unwrap();
            assert!(content.contains("current_theme.conf"));
            assert!(generator.health_check(&ctx).is_ok());
        });
    }

    #[test]
    fn should_fix_broken_link_for_ghostty() {
        let (tmp_dir, ctx) = create_test_context();
        let generator = GhosttyGenerator;
        let p = Palette::mock();
        let root = tmp_dir.path();

        temp_env::with_var("HOME", Some(root), || {
            let config_dir = generator.resolve_config_directory();
            fs::create_dir_all(&config_dir).unwrap();

            generator.apply(&p, &ctx).unwrap();

            let config_path = config_dir.join("config");
            fs::write(&config_path, "config-import = current_theme.conf").unwrap();

            let link_path = generator.link_path("");
            if link_path.exists() || link_path.is_symlink() {
                fs::remove_file(&link_path).unwrap();
            }

            let status = generator.health_check(&ctx);
            assert!(
                matches!(status, HealthStatus::Error { .. }),
                "Expected Error, got {:?}",
                status
            );

            generator.fix(&status, &p, &ctx.silent()).unwrap();

            let final_status = generator.health_check(&ctx);
            assert!(link_path.exists(), "Link should exist");

            match final_status {
                HealthStatus::Error { message, .. } => {
                    panic!("Fix failed, still have Error: {}", message)
                }
                _ => {}
            }
        });
    }
}
