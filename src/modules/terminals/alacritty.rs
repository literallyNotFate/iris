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

/// Config generator for Alacritty terminal
pub struct AlacrittyGenerator;

impl Generator for AlacrittyGenerator {
    fn name(&self) -> &str {
        "alacritty"
    }

    fn generator_type(&self) -> GeneratorType {
        GeneratorType::Terminal
    }

    fn target_file_name(&self, _theme: &str) -> String {
        "current_theme.toml".into()
    }

    fn cache_path(&self, ctx: &IrisContext, _theme_name: &str) -> PathBuf {
        ctx.paths
            .cache
            .join("alacritty")
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
        c.insert("white", &p.white);
        c.insert("sel", &p.sel);
        c.insert("ansi", &p.ansi);
        c
    }

    fn health_check(&self, ctx: &IrisContext) -> HealthStatus {
        if !self.is_installed() {
            return HealthStatus::Warning("Alacritty binary not found".into());
        }

        let alacritty_dir: PathBuf = self.resolve_config_directory();
        let main_config: PathBuf = alacritty_dir.join("alacritty.toml");
        let link_path: PathBuf = self.link_path("");
        let expected_cache: PathBuf = self.cache_path(ctx, "");

        if !link_path.exists() {
            return HealthStatus::Error {
                message: "current_theme.toml missing in config dir".into(),
                fix_hint: Some("run `iris sync` to regenerate".into()),
            };
        }

        #[cfg(unix)]
        if let Ok(target) = std::fs::read_link(&link_path) {
            if target != expected_cache {
                return HealthStatus::Warning("Link points to an old or manual theme file".into());
            }
        }

        if !main_config.exists() {
            return HealthStatus::Warning(
                "alacritty.toml not found (using default settings)".into(),
            );
        }

        let content = fs::read_to_string(&main_config).unwrap_or_default();
        if !content.contains("current_theme.toml") {
            return HealthStatus::Error {
                message: "Theme is not imported in alacritty.toml".into(),
                fix_hint: Some(
                    "Add `import = [\"~/.config/alacritty/current_theme.toml\"]`".into(),
                ),
            };
        }

        HealthStatus::Ok
    }

    fn fix(&self, status: &HealthStatus, p: &Palette, ctx: &IrisContext) -> Result<()> {
        match status {
            HealthStatus::Error { message, .. } => {
                if message.contains("current_theme.toml missing") {
                    ctx.log
                        .step(
                            &format!("Restoring {} theme symlink...", self.name().bold()),
                            2,
                        )
                        .done(true);

                    let cache = self.cache_path(ctx, &p.name);
                    let link = self.link_path(&p.name);
                    self.ensure_symlink(&cache, &link, &ctx.silent())?;
                }

                if message.contains("not imported") {
                    ctx.log
                        .step(
                            &format!("Injecting theme import into {}...", "alacritty.toml".bold()),
                            2,
                        )
                        .done(true);

                    self.inject_import_line()?;
                }

                self.apply(p, &ctx.silent())
            }
            HealthStatus::Warning(msg) if msg.contains("points to an old") => {
                ctx.log
                    .step(
                        &format!("Updating {} symlink target...", self.name().bold()),
                        2,
                    )
                    .done(true);

                self.apply(p, &ctx.silent())
            }
            _ => {
                ctx.log
                    .step(
                        &format!("Syncing {} configuration...", self.name().bold()),
                        2,
                    )
                    .done(true);

                self.apply(p, &ctx.silent())
            }
        }
    }
}

impl AlacrittyGenerator {
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
        let config_path = self.resolve_config_directory().join("alacritty.toml");
        let import_line = "\nimport = [\"~/.config/alacritty/current_theme.toml\"]\n";

        if !config_path.exists() {
            fs::write(&config_path, format!("# Alacritty Config\n{}", import_line))?;
        } else {
            let mut content = fs::read_to_string(&config_path)?;
            if !content.contains("current_theme.toml") {
                content.insert_str(0, &format!("{}\n", import_line.trim()));
                fs::write(&config_path, content)?;
            }
        }
        Ok(())
    }
}

/// Unit-tests for alacritty
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::create_test_context;

    #[test]
    fn should_return_alacritty_metadata() {
        let generator = AlacrittyGenerator;
        assert_eq!(generator.name(), "alacritty");
        assert_eq!(generator.generator_type(), GeneratorType::Terminal);
        assert_eq!(generator.target_file_name("nord"), "current_theme.toml");
    }

    #[test]
    fn should_build_valid_render_context() {
        let generator = AlacrittyGenerator;
        let p = Palette::mock();
        let ctx = generator.build_render_context(&p);

        assert_eq!(
            ctx.get("bg")
                .expect("bg not found in context")
                .as_str()
                .unwrap(),
            p.bg
        );
        assert_eq!(
            ctx.get("fg")
                .expect("fg not found in context")
                .as_str()
                .unwrap(),
            p.fg
        );
        assert!(ctx.contains_key("ansi"));
    }

    #[test]
    fn alacritty_health_ok() {
        let (tmp_dir, mut ctx) = create_test_context();
        let generator = AlacrittyGenerator;
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

                let alacritty_dir = generator.resolve_config_directory();
                let main_config = alacritty_dir.join("alacritty.toml");
                fs::write(&main_config, "import = [\"current_theme.toml\"]").unwrap();

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
    fn alacritty_health_error_no_import() {
        let (tmp_dir, mut ctx) = create_test_context();
        let generator = AlacrittyGenerator;
        let p = Palette::mock();
        let root = tmp_dir.path();

        temp_env::with_vars(vec![("HOME", Some(root.to_str().unwrap()))], || {
            ctx.state.current_theme = p.name.clone();
            generator.apply(&p, &ctx).unwrap();

            let main_config = generator.resolve_config_directory().join("alacritty.toml");
            fs::write(&main_config, "[window]\ndecorations = \"none\"").unwrap();

            let status = generator.health_check(&ctx);
            match status {
                HealthStatus::Error { ref message, .. } => {
                    assert!(message.contains("not imported"));
                }
                _ => panic!("Expected Error for missing import, got {:?}", status),
            }
        });
    }

    #[test]
    fn alacritty_health_warning_no_main_config() {
        let (tmp_dir, mut ctx) = create_test_context();
        let generator = AlacrittyGenerator;
        let p = Palette::mock();
        let root = tmp_dir.path();

        temp_env::with_vars(vec![("HOME", Some(root.to_str().unwrap()))], || {
            ctx.state.current_theme = p.name.clone();
            generator.apply(&p, &ctx).unwrap();

            let main_config = generator.resolve_config_directory().join("alacritty.toml");
            if main_config.exists() {
                fs::remove_file(main_config).unwrap();
            }

            let status = generator.health_check(&ctx);
            assert!(matches!(status, HealthStatus::Warning(msg) if msg.contains("not found")));
        });
    }

    #[test]
    fn should_apply_theme_for_alacritty() {
        if which::which("alacritty").is_err() {
            return;
        }

        let (tmp_dir, ctx) = create_test_context();
        let generator = AlacrittyGenerator;
        let p = Palette::mock();

        temp_env::with_vars(
            vec![
                ("XDG_CONFIG_HOME", Some(tmp_dir.path())),
                ("HOME", Some(tmp_dir.path())),
            ],
            || {
                let result = generator.apply(&p, &ctx);
                assert!(result.is_ok(), "Apply failed: {:?}", result.err());

                let cache_file = ctx.paths.cache.join("alacritty").join("current_theme.toml");
                assert!(cache_file.exists(), "Theme missing in Iris cache");

                let alacritty_dir = generator.resolve_config_directory();
                let link_path = alacritty_dir.join("current_theme.toml");
                assert!(
                    link_path.exists(),
                    "Symlink missing in Alacritty config dir"
                );
                assert!(cache_file.exists());

                let content = fs::read_to_string(cache_file).unwrap();
                assert!(content.contains(&format!("background = \"{}\"", p.bg)));
                assert!(content.contains(&format!("black   = \"{}\"", p.ansi[0])));
                assert!(content.contains(&format!("white   = \"{}\"", p.ansi[15])));
            },
        );
    }

    #[test]
    fn should_fix_inject_issue_for_alacritty() {
        let (tmp_dir, ctx) = create_test_context();
        let generator = AlacrittyGenerator;
        let p = Palette::mock();
        let root = tmp_dir.path();

        let alacritty_dir = root.join(".config/alacritty");
        fs::create_dir_all(&alacritty_dir).unwrap();
        let config_path = alacritty_dir.join("alacritty.toml");
        fs::write(&config_path, "[window]\nopacity = 0.9\n").unwrap();

        temp_env::with_var("HOME", Some(root.to_str().unwrap()), || {
            generator.apply(&p, &ctx).unwrap();

            let status = generator.health_check(&ctx);
            assert!(
                matches!(status, HealthStatus::Error { ref message, .. } if message.contains("not imported"))
            );

            generator
                .fix(&status, &p, &ctx.silent())
                .expect("Fix failed");

            let content = fs::read_to_string(&config_path).unwrap();
            assert!(content.contains("current_theme.toml"));
            assert!(generator.health_check(&ctx).is_ok());
        });
    }

    #[test]
    fn should_fix_broken_symlink_for_alacritty() {
        let (tmp_dir, ctx) = create_test_context();
        let generator = AlacrittyGenerator;
        let p = Palette::mock();
        let root = tmp_dir.path();

        temp_env::with_var("HOME", Some(root.to_str().unwrap()), || {
            generator.apply(&p, &ctx).unwrap();
            let config_path = root.join(".config/alacritty/alacritty.toml");
            fs::write(
                config_path,
                "import = [\"~/.config/alacritty/current_theme.toml\"]",
            )
            .unwrap();

            let link_path = generator.link_path("");
            fs::remove_file(&link_path).unwrap();

            let status = generator.health_check(&ctx);
            assert!(matches!(status, HealthStatus::Error { .. }));

            generator.fix(&status, &p, &ctx.silent()).unwrap();
            assert!(link_path.exists());
            assert!(generator.health_check(&ctx).is_ok());
        });
    }
}
