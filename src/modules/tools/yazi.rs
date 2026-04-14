use crate::{
    commands::HealthStatus,
    core::IrisContext,
    models::Palette,
    modules::{Generator, GeneratorType},
    utils,
};
use anyhow::{Context, Result};
use colored::Colorize;
use std::{fs, path::PathBuf};

/// Config generator for yazi
pub struct YaziGenerator;

impl Generator for YaziGenerator {
    fn name(&self) -> &str {
        "yazi"
    }

    fn generator_type(&self) -> GeneratorType {
        GeneratorType::Tool
    }

    fn target_file_name(&self, _theme: &str) -> String {
        "theme.toml".into()
    }

    fn cache_path(&self, ctx: &IrisContext, _theme_name: &str) -> PathBuf {
        ctx.paths
            .cache
            .join("yazi_themes")
            .join(self.target_file_name(""))
    }

    fn link_path(&self, _theme_name: &str) -> PathBuf {
        self.resolve_config_directory()
            .join(self.target_file_name(""))
    }

    fn apply(&self, p: &Palette, ctx: &IrisContext) -> Result<()> {
        let theme_name: &String = &p.name;
        let cache_file: PathBuf = self.cache_path(ctx, theme_name);
        let theme_link: PathBuf = self.link_path(theme_name);

        let render_ctx = self.build_render_context(p);
        let content: String = ctx.templater.render(&self.template_path(), &render_ctx)?;

        fs::create_dir_all(cache_file.parent().unwrap())?;
        fs::create_dir_all(theme_link.parent().unwrap())?;

        fs::write(&cache_file, content)?;
        ctx.log.info(&format!(
            "Theme {} generated in cache.",
            theme_name.yellow()
        ));

        if theme_link.exists() || theme_link.is_symlink() {
            fs::remove_file(&theme_link)?;
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            ctx.log.info(&format!(
                "Linking {} theme to {}...",
                self.name().bold(),
                utils::pretty_path(&cache_file).cyan(),
            ));
            symlink(&cache_file, &theme_link).with_context(|| {
                format!(
                    "Failed to create symlink {:?} -> {:?}",
                    theme_link, cache_file
                )
            })?;
        }

        Ok(())
    }

    fn build_render_context(&self, p: &Palette) -> tera::Context {
        let mut c = tera::Context::new();

        c.insert("theme_name", &utils::capitalize(&p.name));
        c.insert("bg", &p.bg);
        c.insert("fg", &p.fg);
        c.insert("white", &p.white);
        c.insert("comment", &p.comment);
        c.insert("gutter_fg", &p.gutter_fg);
        c.insert("ansi", &p.ansi);
        c.insert("sel", &p.sel);

        let line_hl = if p.line_hl == "#cccccc" {
            &p.sel
        } else {
            &p.line_hl
        };
        c.insert("line_hl", line_hl);

        c.insert("red", &p.ansi[1]);
        c.insert("green", &p.ansi[2]);
        c.insert("orange", &p.ansi[3]);
        c.insert("blue", &p.ansi[4]);
        c.insert("magenta", &p.ansi[5]);
        c.insert("teal", &p.ansi[6]);
        c.insert("tan", &p.ansi[7]);
        c.insert("br_red", &p.ansi[9]);
        c.insert("br_green", &p.ansi[10]);
        c.insert("br_orange", &p.ansi[11]);
        c.insert("br_blue", &p.ansi[12]);
        c.insert("br_magenta", &p.ansi[13]);
        c.insert("br_teal", &p.ansi[14]);

        c
    }

    fn health_check(&self, ctx: &IrisContext) -> HealthStatus {
        if !self.is_installed() {
            return HealthStatus::Warning("Yazi binary not found".into());
        }

        let link_path: PathBuf = self.link_path("");
        let expected_cache: PathBuf = self.cache_path(ctx, "");

        if !link_path.exists() && !link_path.is_symlink() {
            return HealthStatus::Error {
                message: "theme.toml link missing in yazi config".into(),
                fix_hint: Some("run `iris sync` to create the symlink".into()),
            };
        }

        #[cfg(unix)]
        if let Ok(target) = std::fs::read_link(&link_path) {
            if target != expected_cache {
                return HealthStatus::Warning(format!(
                    "Yazi theme link points to an unexpected location: {:?}",
                    target
                ));
            }
        }

        if !expected_cache.exists() {
            return HealthStatus::Error {
                message: "Yazi theme cache file is missing".into(),
                fix_hint: Some("run `iris sync` to regenerate".into()),
            };
        }

        HealthStatus::Ok
    }
}

/// Unit-tests for yazi generator
#[cfg(test)]
mod tests {
    #[cfg(unix)]
    use std::os::unix::fs::symlink;

    use super::*;
    use crate::test_utils::create_test_context;
    use temp_env;

    #[test]
    fn should_return_yazi_metadata() {
        let generator = YaziGenerator;
        assert_eq!(generator.name(), "yazi");
        assert_eq!(generator.generator_type(), GeneratorType::Tool);
        assert_eq!(generator.target_file_name("any"), "theme.toml");
    }

    #[test]
    fn should_build_valid_render_context() {
        let generator = YaziGenerator;
        let mut p = Palette::mock();

        p.line_hl = "#123456".to_string();
        let ctx = generator.build_render_context(&p);
        assert_eq!(ctx.get("line_hl").unwrap().as_str().unwrap(), "#123456");

        p.line_hl = "#cccccc".to_string();
        p.sel = "#ff0000".to_string();
        let ctx = generator.build_render_context(&p);

        assert_eq!(ctx.get("line_hl").unwrap().as_str().unwrap(), "#ff0000");
        assert!(ctx.get("red").is_some());
        assert!(ctx.get("br_teal").is_some());
    }

    #[test]
    fn yazi_health_ok() {
        let (tmp_dir, mut ctx) = create_test_context();
        let generator = YaziGenerator;
        let p = Palette::mock();
        let root = tmp_dir.path();

        temp_env::with_var("HOME", Some(root), || {
            ctx.state.current_theme = p.name.clone();
            generator.apply(&p, &ctx).unwrap();

            let status = generator.health_check(&ctx);
            assert!(matches!(status, HealthStatus::Ok));
        });
    }

    #[test]
    fn yazi_health_error_missing_link() {
        let (tmp_dir, ctx) = create_test_context();
        let generator = YaziGenerator;
        let root = tmp_dir.path();

        temp_env::with_var("HOME", Some(root), || {
            let status = generator.health_check(&ctx);

            match status {
                HealthStatus::Error { message, .. } => {
                    assert!(message.contains("link missing"));
                }
                _ => panic!("Expected Error, got {:?}", status),
            }
        });
    }

    #[test]
    fn yazi_health_warning_wrong_target() {
        let (tmp_dir, mut ctx) = create_test_context();
        let generator = YaziGenerator;
        let p = Palette::mock();
        let root = tmp_dir.path();

        temp_env::with_var("HOME", Some(root), || {
            ctx.state.current_theme = p.name.clone();
            generator.apply(&p, &ctx).unwrap();

            let link_path = generator.link_path(&p.name);
            let wrong_target = root.join("some_other_place.toml");
            fs::write(&wrong_target, "").unwrap();

            fs::remove_file(&link_path).unwrap();
            #[cfg(unix)]
            symlink(&wrong_target, &link_path).unwrap();

            let status = generator.health_check(&ctx);
            assert!(matches!(status, HealthStatus::Warning(_)));
        });
    }

    #[test]
    fn yazi_health_error_missing_cache() {
        let (tmp_dir, mut ctx) = create_test_context();
        let generator = YaziGenerator;
        let p = Palette::mock();
        let root = tmp_dir.path();

        temp_env::with_var("HOME", Some(root), || {
            ctx.state.current_theme = p.name.clone();
            generator.apply(&p, &ctx).unwrap();

            let cache_path = generator.cache_path(&ctx, &p.name);
            fs::remove_file(cache_path).unwrap();

            let status = generator.health_check(&ctx);
            match status {
                HealthStatus::Error { message, .. } => {
                    assert!(message.contains("cache file is missing"));
                }
                _ => panic!("Expected Error, got {:?}", status),
            }
        });
    }

    #[test]
    fn should_apply_theme_for_yazi() {
        if which::which("yazi").is_err() {
            return;
        }

        let (tmp_dir, ctx) = create_test_context();
        let generator = YaziGenerator;
        let p = Palette::mock();

        temp_env::with_vars(
            vec![
                ("XDG_CONFIG_HOME", Some(tmp_dir.path())),
                ("HOME", Some(tmp_dir.path())),
            ],
            || {
                let result = generator.apply(&p, &ctx);
                assert!(result.is_ok(), "Apply failed: {:?}", result.err());

                let expected_yazi_dir = generator.resolve_config_directory();
                let yazi_theme_link = expected_yazi_dir.join("theme.toml");

                assert!(
                    yazi_theme_link.exists(),
                    "Symlink missing at {:?}. Check if resolve_config_directory is consistent!",
                    yazi_theme_link
                );

                let cache_content = fs::read_to_string(yazi_theme_link).unwrap();
                assert!(cache_content.contains("generated by Iris"));
            },
        );
    }
}
