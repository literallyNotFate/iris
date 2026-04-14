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

/// Config generator for tmux
pub struct TmuxGenerator;

impl Generator for TmuxGenerator {
    fn name(&self) -> &str {
        "tmux"
    }

    fn generator_type(&self) -> GeneratorType {
        GeneratorType::Multiplexer
    }

    fn target_file_name(&self, theme: &str) -> String {
        format!("{}.conf", theme)
    }

    fn cache_path(&self, ctx: &IrisContext, theme_name: &str) -> PathBuf {
        ctx.paths
            .cache
            .join("tmux_themes")
            .join(self.target_file_name(theme_name))
    }

    fn link_path(&self, theme_name: &str) -> PathBuf {
        self.resolve_config_directory()
            .join(self.target_file_name(theme_name))
    }

    fn resolve_config_directory(&self) -> PathBuf {
        dirs::home_dir()
            .map(|p| p.join(".config").join("tmux").join("themes"))
            .unwrap_or_else(|| PathBuf::from(".config/tmux/themes"))
    }

    fn apply(&self, p: &Palette, ctx: &IrisContext) -> Result<()> {
        let theme_name: &String = &p.name;
        let cache_file: PathBuf = self.cache_path(ctx, theme_name);
        let theme_link: PathBuf = self.link_path(theme_name);
        let themes_dir: PathBuf = self.resolve_config_directory();

        let render_ctx = self.build_render_context(p);
        let content: String = ctx.templater.render(&self.template_path(), &render_ctx)?;

        fs::create_dir_all(cache_file.parent().unwrap())?;
        if !themes_dir.exists() {
            fs::create_dir_all(&themes_dir)?;
        }

        fs::write(&cache_file, content)?;

        if theme_link.exists() || theme_link.is_symlink() {
            fs::remove_file(&theme_link)?;
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            ctx.log.info(&format!(
                "Linking {} theme to {}...",
                self.name().bold(),
                utils::pretty_path(&cache_file).cyan()
            ));
            symlink(&cache_file, &theme_link).with_context(|| {
                format!(
                    "Failed to create symlink {:?} -> {:?}",
                    theme_link, cache_file
                )
            })?;
        }

        let tmux_root = themes_dir.parent().unwrap_or(&themes_dir);
        let conf_path: PathBuf = tmux_root.join("tmux.conf");

        if conf_path.exists() {
            ctx.log.info(&format!(
                "Patching {} to source theme {}",
                self.target_file_name("").bold(),
                utils::capitalize(theme_name).yellow()
            ));
            self.update_tmux_conf(&conf_path, theme_name)?;
        } else {
            ctx.log
                .warn("tmux.conf not found. Theme linked but not sourced.", 3);
        }

        Ok(())
    }

    fn build_render_context(&self, p: &Palette) -> tera::Context {
        let mut c = tera::Context::new();

        c.insert("theme_name", &utils::capitalize(&p.name));
        c.insert("bg", &p.bg);
        c.insert("fg", &p.fg);
        c.insert("keyword", &p.keyword);
        c.insert("comment", &p.comment);
        c.insert("operator", &p.operator);
        c.insert("gutter_fg", &p.gutter_fg);
        c.insert("line_hl", &p.line_hl);
        c.insert("func", &p.func);
        c.insert("tag", &p.tag);

        c.insert("green", &p.ansi[10]);
        c.insert("yellow", &p.ansi[3]);
        c.insert("blue", &p.ansi[12]);

        c
    }

    fn health_check(&self, ctx: &IrisContext) -> HealthStatus {
        if !self.is_installed() {
            return HealthStatus::Warning("tmux binary not found".into());
        }

        let themes_dir: PathBuf = self.resolve_config_directory();
        let tmux_conf: PathBuf = themes_dir.parent().unwrap_or(&themes_dir).join("tmux.conf");

        if !tmux_conf.exists() {
            return HealthStatus::Error {
                message: "tmux.conf missing".into(),
                fix_hint: Some("Create ~/.config/tmux/tmux.conf".into()),
            };
        }

        let theme: &String = &ctx.state.current_theme;

        if !theme.is_empty() {
            let content: String = fs::read_to_string(&tmux_conf).unwrap_or_default();
            let marker = "# iris-theme";

            if !content.contains(marker) {
                return HealthStatus::Warning(format!(
                    "Iris theme is not sourced in {}. Run `iris sync`.",
                    tmux_conf.display()
                ));
            }

            let expected_file = format!("{}.conf", theme);
            if !content.contains(&expected_file) {
                return HealthStatus::Warning(format!(
                    "tmux.conf sources a different theme, not '{}'",
                    theme
                ));
            }

            let link: PathBuf = self.link_path(theme);
            if !link.exists() {
                return HealthStatus::Error {
                    message: format!("Theme link missing: {}", link.display()),
                    fix_hint: Some("Run `iris sync` to restore symlink".into()),
                };
            }
        }

        HealthStatus::Ok
    }
}

impl TmuxGenerator {
    /// Ensure tmux.conf sources the iris theme file.
    /// Replaces an existing Iris source line or appends one before the `run` line (tpm).
    fn update_tmux_conf(&self, path: &PathBuf, theme_name: &str) -> Result<()> {
        if !path.exists() {
            return Ok(());
        }

        let source_line = format!(
            "source-file \"~/.config/tmux/themes/{}.conf\" # iris-theme",
            theme_name
        );

        let content = fs::read_to_string(path)?;
        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        let mut updated = false;

        for line in lines.iter_mut() {
            if line.contains("# iris-theme") {
                *line = source_line.clone();
                updated = true;
                break;
            }
        }

        if !updated {
            let run_pos = lines
                .iter()
                .position(|l| l.trim_start().starts_with("run ") && l.contains("tpm"));

            match run_pos {
                Some(pos) => lines.insert(pos, source_line),
                None => lines.push(source_line),
            }
        }

        fs::write(path, lines.join("\n"))?;
        Ok(())
    }
}

/// Unit-tests for tmux generator
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::create_test_context;
    use tempdir::TempDir;

    #[test]
    fn should_return_tmux_metadata() {
        let generator = TmuxGenerator;
        assert_eq!(generator.name(), "tmux");
        assert_eq!(generator.generator_type(), GeneratorType::Multiplexer);
        assert_eq!(generator.target_file_name("dracula"), "dracula.conf");
    }

    #[test]
    fn should_build_valid_render_context() {
        let generator = TmuxGenerator;
        let p = Palette::mock();
        let ctx = generator.build_render_context(&p);

        assert_eq!(ctx.get("bg").unwrap().as_str().unwrap(), p.bg);
    }

    #[test]
    fn should_handle_update_tmux_conf_logic() {
        let generator = TmuxGenerator;
        let temp_dir: TempDir = TempDir::new("tmux_test").unwrap();
        let conf_path = temp_dir.path().join("tmux.conf");

        fs::write(
            &conf_path,
            "set -g prefix C-a\nsource-file \"old.conf\" # iris-theme\n",
        )
        .unwrap();
        generator.update_tmux_conf(&conf_path, "new-theme").unwrap();
        let content = fs::read_to_string(&conf_path).unwrap();
        assert!(content.contains("new-theme.conf\" # iris-theme"));
        assert!(!content.contains("old.conf"));

        fs::write(
            &conf_path,
            "set -g base-index 1\nrun '~/.tmux/plugins/tpm/tpm'",
        )
        .unwrap();
        generator.update_tmux_conf(&conf_path, "theme-x").unwrap();
        let content = fs::read_to_string(&conf_path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(
            lines[1],
            "source-file \"~/.config/tmux/themes/theme-x.conf\" # iris-theme"
        );
    }

    #[test]
    fn tmux_health_ok() {
        let (tmp_dir, mut ctx) = create_test_context();
        let generator = TmuxGenerator;
        let p = Palette::mock();
        let root = tmp_dir.path();

        let tmux_dir = root.join(".config").join("tmux");
        fs::create_dir_all(&tmux_dir).unwrap();

        let tmux_conf = tmux_dir.join("tmux.conf");
        fs::write(&tmux_conf, "# initial config").unwrap();

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
                generator.apply(&p, &ctx).expect("Apply failed");
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
    fn tmux_health_warning_missing_marker() {
        let (tmp_dir, mut ctx) = create_test_context();
        let generator = TmuxGenerator;
        let p = Palette::mock();
        let root = tmp_dir.path();

        let tmux_dir = root.join(".config").join("tmux");
        fs::create_dir_all(&tmux_dir).unwrap();
        let tmux_conf = tmux_dir.join("tmux.conf");

        temp_env::with_var("HOME", Some(root.to_str().unwrap()), || {
            ctx.state.current_theme = p.name.clone();
            fs::write(&tmux_conf, "set -g mouse on").unwrap();

            let status = generator.health_check(&ctx);
            assert!(
                matches!(&status, HealthStatus::Warning(msg) if msg.contains("not sourced")),
                "Expected Warning for missing marker, got {:?}",
                status
            );
        });
    }

    #[test]
    fn tmux_health_warning_wrong_theme_sourced() {
        let (tmp_dir, mut ctx) = create_test_context();
        let generator = TmuxGenerator;
        let p = Palette::mock();
        let root = tmp_dir.path();

        let tmux_dir = root.join(".config").join("tmux");
        fs::create_dir_all(&tmux_dir).unwrap();
        let tmux_conf = tmux_dir.join("tmux.conf");

        temp_env::with_var("HOME", Some(root.to_str().unwrap()), || {
            ctx.state.current_theme = p.name.clone();
            fs::write(
                &tmux_conf,
                "source-file ~/.config/tmux/themes/wrong.conf # iris-theme",
            )
            .unwrap();

            let status = generator.health_check(&ctx);
            assert!(
                matches!(&status, HealthStatus::Warning(msg) if msg.contains("sources a different theme")),
                "Expected Warning for wrong theme, got {:?}",
                status
            );
        });
    }

    #[test]
    fn tmux_health_error_link_missing() {
        let (tmp_dir, mut ctx) = create_test_context();
        let generator = TmuxGenerator;
        let p = Palette::mock();
        let root = tmp_dir.path();

        let tmux_dir = root.join(".config").join("tmux");
        fs::create_dir_all(&tmux_dir).unwrap();
        fs::write(tmux_dir.join("tmux.conf"), "").unwrap();

        temp_env::with_var("HOME", Some(root.to_str().unwrap()), || {
            ctx.state.current_theme = p.name.clone();

            generator.apply(&p, &ctx).unwrap();

            let link = generator.link_path(&p.name);
            if link.exists() {
                fs::remove_file(link).unwrap();
            }

            let status = generator.health_check(&ctx);
            assert!(
                matches!(status, HealthStatus::Error { ref message, .. } if message.contains("Theme link missing")),
                "Expected Theme link missing, got {:?}",
                status
            );
        });
    }

    #[test]
    fn should_apply_theme_and_patch_tmux_conf() {
        if which::which("tmux").is_err() {
            return;
        }

        let (tmp_dir, ctx) = create_test_context();
        let generator = TmuxGenerator;
        let p = Palette::mock();

        temp_env::with_vars(
            vec![
                ("XDG_CONFIG_HOME", Some(tmp_dir.path().to_str().unwrap())),
                ("HOME", Some(tmp_dir.path().to_str().unwrap())),
            ],
            || {
                let tmux_dir = generator
                    .resolve_config_directory()
                    .parent()
                    .expect("Failed to get tmux config parent dir")
                    .to_path_buf();
                let tmux_conf = tmux_dir.join("tmux.conf");

                fs::create_dir_all(&tmux_dir).unwrap();
                fs::write(
                    &tmux_conf,
                    "set -g mouse on\n\nrun '~/.tmux/plugins/tpm/tpm'",
                )
                .unwrap();

                let result = generator.apply(&p, &ctx);
                if let Err(e) = &result {
                    eprintln!("Tmux Apply Error: {:?}", e);
                }
                assert!(
                    result.is_ok(),
                    "Tmux apply should return Ok. Check if all templates are valid."
                );

                let cache_file = ctx
                    .paths
                    .cache
                    .join("tmux_themes")
                    .join(format!("{}.conf", p.name));
                assert!(
                    cache_file.exists(),
                    "Cache file not found at {:?}",
                    cache_file
                );

                let updated_content = fs::read_to_string(&tmux_conf).unwrap();

                let _expected_line = format!(
                    "source-file \"~/.config/tmux/themes/{}.conf\" # iris-theme",
                    p.name
                );
                assert!(
                    updated_content.contains("# iris-theme"),
                    "tmux.conf should contain iris-theme marker"
                );

                let lines: Vec<&str> = updated_content.lines().collect();
                let theme_idx = lines
                    .iter()
                    .position(|l| l.contains("# iris-theme"))
                    .expect("Could not find iris-theme line in tmux.conf");

                let tpm_idx = lines
                    .iter()
                    .position(|l| l.contains("tpm"))
                    .expect("Could not find TPM line in tmux.conf");

                assert!(
                    theme_idx < tpm_idx,
                    "Theme should be sourced before TPM to allow overrides"
                );
            },
        );
    }
}
