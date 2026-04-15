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

/// Config generator for btop utility
pub struct BtopGenerator;

impl Generator for BtopGenerator {
    fn name(&self) -> &str {
        "btop"
    }

    fn generator_type(&self) -> GeneratorType {
        GeneratorType::System
    }

    fn target_file_name(&self, theme: &str) -> String {
        format!("{}.theme", theme)
    }

    fn resolve_config_directory(&self) -> PathBuf {
        dirs::home_dir()
            .map(|p| p.join(".config").join("btop").join("themes"))
            .unwrap_or_else(|| PathBuf::from(".config/btop/themes"))
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

        let conf_path: PathBuf = self
            .resolve_config_directory()
            .parent()
            .unwrap_or(&self.resolve_config_directory())
            .join("btop.conf");

        if conf_path.exists() {
            ctx.log.info(&format!(
                "Setting color_theme = \"{}\" in btop.conf",
                p.name.bold().red()
            ));

            self.update_btop_conf(&conf_path, &p.name)?;
        }

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
        c.insert("sel", &p.sel);
        c.insert("white", &p.white);
        c.insert("comment", &p.comment);
        c.insert("line_hl", &p.line_hl);
        c.insert("keyword", &p.keyword);
        c.insert("type_name", &p.type_name);
        c.insert("func", &p.func);
        c.insert("tag", &p.tag);
        c.insert("string", &p.string);
        c.insert("constant", &p.constant);
        c.insert("attribute", &p.attribute);

        c.insert("green", &p.ansi[2]);
        c.insert("yellow", &p.ansi[3]);
        c.insert("orange", &p.ansi[9]);

        c
    }

    fn health_check(&self, ctx: &IrisContext) -> HealthStatus {
        if !self.is_installed() {
            return HealthStatus::Warning("btop binary not found".into());
        }

        let themes_dir: PathBuf = self.resolve_config_directory();
        let conf_path: PathBuf = themes_dir.parent().unwrap_or(&themes_dir).join("btop.conf");

        if !conf_path.exists() {
            return HealthStatus::Error {
                message: "btop.conf missing".into(),
                fix_hint: Some("Run btop once to generate default config".into()),
            };
        }

        let theme: &String = &ctx.state.current_theme;

        if !theme.is_empty() {
            let content: String = fs::read_to_string(&conf_path).unwrap_or_default();
            let expected_line = format!("color_theme = \"{}\"", theme);

            if !content.contains(&expected_line) {
                return HealthStatus::Warning(format!(
                    "btop.conf is not using the current theme '{}'",
                    theme
                ));
            }

            let link = self.link_path(theme);
            if !link.exists() {
                return HealthStatus::Error {
                    message: format!("Theme file {}.theme missing in btop themes folder", theme),
                    fix_hint: Some("Run `iris sync` to restore the theme link".into()),
                };
            }
        }

        HealthStatus::Ok
    }

    fn fix(&self, status: &HealthStatus, p: &Palette, ctx: &IrisContext) -> Result<()> {
        match status {
            HealthStatus::Error { message, .. } => {
                if message.contains("missing") {
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

                self.apply(p, &ctx.silent())
            }

            HealthStatus::Warning(msg) if msg.contains("not using the current theme") => {
                ctx.log
                    .step("Updating btop.conf to use the correct theme...", 2)
                    .done(true);

                let conf_path = self
                    .resolve_config_directory()
                    .parent()
                    .unwrap_or(&self.resolve_config_directory())
                    .join("btop.conf");

                self.update_btop_conf(&conf_path, &p.name)
            }

            _ => {
                ctx.log
                    .step(
                        &format!("Refreshing {} configuration...", self.name().bold()),
                        2,
                    )
                    .done(true);
                self.apply(p, &ctx.silent())
            }
        }
    }
}

impl BtopGenerator {
    /// Update color_theme setting in btop.conf
    fn update_btop_conf(&self, path: &PathBuf, name: &str) -> Result<()> {
        if !path.exists() {
            return Ok(());
        }

        let content = fs::read_to_string(path)?;
        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        let mut updated = false;

        let theme_line = format!("color_theme = \"{}\"", name);

        for line in lines.iter_mut() {
            if line.trim_start().starts_with("color_theme =") {
                *line = theme_line.clone();
                updated = true;
                break;
            }
        }

        if !updated {
            lines.push(theme_line);
        }

        fs::write(path, lines.join("\n"))?;
        Ok(())
    }

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
}

/// Unit-tests for btop generator
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::tests::create_test_context;
    use tempdir::TempDir;

    #[test]
    fn should_return_btop_metadata() {
        let generator = BtopGenerator;
        assert_eq!(generator.name(), "btop");
        assert_eq!(generator.generator_type(), GeneratorType::System);
        assert_eq!(generator.target_file_name("iris-dark"), "iris-dark.theme");
    }

    #[test]
    fn should_build_valid_render_context() {
        let generator = BtopGenerator;
        let p = Palette::mock();
        let ctx = generator.build_render_context(&p);

        assert_eq!(ctx.get("bg").expect("bg missing").as_str().unwrap(), p.bg);
        assert_eq!(ctx.get("fg").expect("fg missing").as_str().unwrap(), p.fg);
        assert_eq!(
            ctx.get("keyword")
                .expect("keyword missing")
                .as_str()
                .unwrap(),
            p.keyword
        );

        assert!(ctx.contains_key("green"));
        assert!(ctx.contains_key("yellow"));
        assert!(ctx.contains_key("orange"));
        assert!(ctx.contains_key("type_name"));
        assert!(ctx.contains_key("theme_name"));
    }

    #[test]
    fn should_update_existing_line_or_append() {
        let generator = BtopGenerator;
        let temp_dir: TempDir = TempDir::new("btop_test").unwrap();
        let conf_path = temp_dir.path().join("btop.conf");

        fs::write(
            &conf_path,
            "theme_background = True\ncolor_theme = \"default\"\n",
        )
        .unwrap();
        generator.update_btop_conf(&conf_path, "new-theme").unwrap();
        let content = fs::read_to_string(&conf_path).unwrap();
        assert!(content.contains("color_theme = \"new-theme\""));
        assert!(!content.contains("color_theme = \"default\""));

        fs::write(&conf_path, "theme_background = True\n").unwrap();
        generator
            .update_btop_conf(&conf_path, "only-theme")
            .unwrap();
        let content = fs::read_to_string(&conf_path).unwrap();
        assert!(content.contains("color_theme = \"only-theme\""));
    }

    #[test]
    fn btop_health_ok() {
        let (tmp_dir, mut ctx) = create_test_context();
        let generator = BtopGenerator;
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

                let btop_dir = generator
                    .resolve_config_directory()
                    .parent()
                    .unwrap()
                    .to_path_buf();
                fs::create_dir_all(&btop_dir).unwrap();
                let conf_path = btop_dir.join("btop.conf");

                let expected_line = format!("color_theme = \"{}\"", p.name);
                fs::write(
                    &conf_path,
                    format!("graph_symbol = \"braille\"\n{}", expected_line),
                )
                .unwrap();

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
    fn btop_health_error_missing_conf() {
        let (tmp_dir, ctx) = create_test_context();
        let generator = BtopGenerator;
        let root = tmp_dir.path();

        temp_env::with_vars(vec![("HOME", Some(root.to_str().unwrap()))], || {
            let status = generator.health_check(&ctx);
            match status {
                HealthStatus::Error { ref message, .. } => {
                    assert!(message.contains("btop.conf missing"));
                }
                _ => panic!("Expected Error for missing btop.conf, got {:?}", status),
            }
        });
    }

    #[test]
    fn btop_health_warning_wrong_theme_in_conf() {
        let (tmp_dir, mut ctx) = create_test_context();
        let generator = BtopGenerator;
        let p = Palette::mock();
        let root = tmp_dir.path();

        temp_env::with_vars(vec![("HOME", Some(root.to_str().unwrap()))], || {
            ctx.state.current_theme = p.name.clone();
            generator.apply(&p, &ctx).unwrap();

            let btop_dir = generator
                .resolve_config_directory()
                .parent()
                .unwrap()
                .to_path_buf();
            fs::create_dir_all(&btop_dir).unwrap();
            fs::write(btop_dir.join("btop.conf"), "color_theme = \"default\"").unwrap();

            let status = generator.health_check(&ctx);
            match status {
                HealthStatus::Warning(msg) => {
                    assert!(msg.contains("not using the current theme"));
                }
                _ => panic!("Expected Warning for wrong theme line, got {:?}", status),
            }
        });
    }

    #[test]
    fn should_apply_theme_and_update_conf() {
        let (tmp_dir, ctx) = create_test_context();
        let generator = BtopGenerator;
        let p = Palette::mock();

        temp_env::with_vars(
            vec![
                ("XDG_CONFIG_HOME", Some(tmp_dir.path())),
                ("HOME", Some(tmp_dir.path())),
            ],
            || {
                let btop_dir = generator
                    .resolve_config_directory()
                    .parent()
                    .unwrap()
                    .to_path_buf();
                let btop_conf = btop_dir.join("btop.conf");

                fs::create_dir_all(&btop_dir).unwrap();
                fs::write(
                    &btop_conf,
                    "graph_symbol = \"braille\"\ncolor_theme = \"old-theme\"\n",
                )
                .unwrap();

                let result = generator.apply(&p, &ctx);
                assert!(result.is_ok());

                let cache_file = ctx.paths.generators.join("btop").join("test-theme.theme");
                assert!(cache_file.exists());

                let updated_content = fs::read_to_string(&btop_conf).unwrap();
                assert!(updated_content.contains(&format!("color_theme = \"{}\"", p.name)));
                assert!(updated_content.contains("graph_symbol = \"braille\""));
            },
        );
    }

    #[test]
    fn should_fix_broken_conf_for_btop() {
        let (tmp_dir, ctx) = create_test_context();
        let generator = BtopGenerator;
        let p = Palette::mock();
        let root = tmp_dir.path();

        temp_env::with_var("HOME", Some(root.to_str().unwrap()), || {
            let btop_dir = root.join(".config/btop");
            fs::create_dir_all(&btop_dir).unwrap();
            let conf_path = btop_dir.join("btop.conf");
            fs::write(
                &conf_path,
                "color_theme = \"wrong_theme\"\nother_setting = true",
            )
            .unwrap();

            let status = generator.health_check(&ctx);

            if let HealthStatus::Warning(msg) = &status {
                assert!(msg.contains("not using the current theme"));

                generator
                    .fix(&status, &p, &ctx.silent())
                    .expect("Fix failed");

                let content = fs::read_to_string(&conf_path).unwrap();
                assert!(content.contains(&format!("color_theme = \"{}\"", p.name)));
                assert!(content.contains("other_setting = true"));
            }
        });
    }

    #[test]
    fn should_fix_missing_theme_file_for_btop() {
        let (tmp_dir, mut ctx) = create_test_context();
        let generator = BtopGenerator;
        let p = Palette::mock();
        let root = tmp_dir.path();

        temp_env::with_var("HOME", Some(root.to_str().unwrap()), || {
            ctx.state.current_theme = p.name.clone();
            let btop_dir = root.join(".config/btop");
            fs::create_dir_all(btop_dir.join("themes")).unwrap();

            fs::write(
                btop_dir.join("btop.conf"),
                format!("color_theme = \"{}\"", p.name),
            )
            .unwrap();

            generator.apply(&p, &ctx).unwrap();

            let link_path = generator.link_path(&p.name);
            assert!(link_path.exists());

            fs::remove_file(&link_path).unwrap();

            let status = generator.health_check(&ctx);
            assert!(
                matches!(status, HealthStatus::Error { .. }),
                "Expected Error due to missing theme file, got {:?}",
                status
            );

            generator.fix(&status, &p, &ctx.silent()).unwrap();
            assert!(link_path.exists(), "Fix should restore the symlink");
        });
    }
}
