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

    fn resolve_config_directory(&self, ctx: &IrisContext) -> PathBuf {
        let config_base: PathBuf = ctx
            .paths
            .config
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| ctx.paths.config.clone());

        config_base.join(self.name()).join("themes")
    }

    fn apply(&self, p: &Palette, ctx: &IrisContext) -> Result<()> {
        ctx.log.info(&format!(
            "Generating {} theme for {}",
            utils::capitalize(&p.name).yellow(),
            self.name().bold().cyan(),
        ));

        let cache_file = self.ensure_cache_file(p, ctx)?;
        let link_path = self.link_path(ctx, &p.name);

        ctx.log.info(&format!(
            "Linking {} theme to {}...",
            self.name().bold(),
            utils::pretty_path(&link_path).magenta(),
        ));
        self.ensure_symlink(&cache_file, &link_path, ctx)?;

        let conf_path = self.resolve_tmux_conf_path(ctx);

        if conf_path.exists() {
            ctx.log.info(&format!(
                "Patching tmux.conf to source {}...",
                utils::capitalize(&p.name).yellow(),
            ));
            self.update_tmux_conf(&conf_path, &p.name)?;
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

        let tmux_conf: PathBuf = self.resolve_tmux_conf_path(ctx);

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

            let link: PathBuf = self.link_path(ctx, theme);
            if !link.exists() {
                return HealthStatus::Error {
                    message: format!("Theme link missing: {}", link.display()),
                    fix_hint: Some("Run `iris sync` to restore symlink".into()),
                };
            }
        }

        HealthStatus::Ok
    }

    fn fix(&self, status: &HealthStatus, p: &Palette, ctx: &IrisContext) -> Result<()> {
        match status {
            HealthStatus::Error { message, .. } if message.contains("Theme link missing") => {
                ctx.log
                    .step(
                        &format!("Restoring {} theme symlink...", self.name().bold()),
                        2,
                    )
                    .done(true);

                self.apply(p, &ctx.silent())
            }

            HealthStatus::Warning(msg)
                if msg.contains("not sourced") || msg.contains("different theme") =>
            {
                ctx.log
                    .step("Repairing tmux.conf source line...", 2)
                    .done(true);

                let conf_path = self.resolve_tmux_conf_path(ctx);
                self.update_tmux_conf(&conf_path, &p.name)
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
                .position(|l| l.trim().starts_with("run ") && l.contains("tpm"));

            match run_pos {
                Some(pos) => lines.insert(pos, source_line),
                None => lines.push(source_line),
            }
        }

        let new_content = lines.join("\n");
        fs::write(path, new_content)?;
        Ok(())
    }

    fn resolve_tmux_conf_path(&self, ctx: &IrisContext) -> PathBuf {
        let themes_dir: PathBuf = self.resolve_config_directory(ctx);

        themes_dir
            .parent()
            .unwrap_or(&themes_dir)
            .to_path_buf()
            .join("tmux.conf")
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

/// Unit-tests for tmux generator
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::tests::create_test_context;
    use tempdir::TempDir;

    // Helper function to get tmux conf just like in generator
    fn get_tmux_conf_path(ctx: &IrisContext) -> PathBuf {
        let generator = TmuxGenerator;
        let themes_dir = generator.resolve_config_directory(ctx);
        themes_dir.parent().unwrap_or(&themes_dir).join("tmux.conf")
    }

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
    fn should_return_health_ok_for_tmux() {
        let (tmp_dir, mut ctx) = create_test_context();
        let generator = TmuxGenerator;
        let p = Palette::mock();
        let root = tmp_dir.path();

        let tmux_dir = root.join(".config").join("tmux");
        let themes_dir = tmux_dir.join("themes");
        let tmux_conf = tmux_dir.join("tmux.conf");

        fs::create_dir_all(&themes_dir).unwrap();
        fs::write(
            &tmux_conf,
            format!(
                "source-file \"~/.config/tmux/themes/{}.conf\" # iris-theme",
                p.name
            ),
        )
        .unwrap();

        ctx.state.current_theme = p.name.clone();
        generator.apply(&p, &ctx).expect("Apply failed");

        let status = generator.health_check(&ctx);
        assert!(
            matches!(status, HealthStatus::Ok),
            "Expected Ok, got {:?}",
            status
        );
    }

    #[test]
    fn should_return_health_warning_missing_marker_for_tmux() {
        let (tmp_dir, mut ctx) = create_test_context();
        let generator = TmuxGenerator;
        let p = Palette::mock();
        let root = tmp_dir.path();

        let tmux_dir = root.join(".config").join("tmux");
        fs::create_dir_all(&tmux_dir).unwrap();

        let tmux_conf = tmux_dir.join("tmux.conf");
        fs::write(
            &tmux_conf,
            format!(
                "source-file \"~/.config/tmux/themes/{}.conf\" # iris-theme",
                p.name
            ),
        )
        .unwrap();

        ctx.state.current_theme = p.name.clone();
        generator.apply(&p, &ctx).unwrap();

        let link = generator.link_path(&ctx, &p.name);
        if link.exists() {
            fs::remove_file(&link).unwrap();
        }

        let status = generator.health_check(&ctx);
        assert!(
            matches!(status, HealthStatus::Error { ref message, .. } if message.contains("Theme link missing")),
            "Expected Theme link missing, got {:?}",
            status
        );
    }

    #[test]
    fn should_return_health_warning_wrong_theme_sourced_for_tmux() {
        let (tmp_dir, mut ctx) = create_test_context();
        let generator = TmuxGenerator;
        let p = Palette::mock();
        let root = tmp_dir.path();

        let tmux_dir = root.join(".config").join("tmux");
        fs::create_dir_all(&tmux_dir).unwrap();
        let tmux_conf = tmux_dir.join("tmux.conf");

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
    }

    #[test]
    fn should_return_health_error_link_missing_for_tmux() {
        let (tmp_dir, mut ctx) = create_test_context();
        let generator = TmuxGenerator;
        let p = Palette::mock();
        let root = tmp_dir.path();

        let tmux_dir = root.join(".config").join("tmux");
        let tmux_conf = tmux_dir.join("tmux.conf");
        fs::create_dir_all(&tmux_dir).unwrap();
        fs::write(
            &tmux_conf,
            format!(
                "source-file \"~/.config/tmux/themes/{}.conf\" # iris-theme",
                p.name
            ),
        )
        .unwrap();

        ctx.state.current_theme = p.name.clone();
        generator.apply(&p, &ctx).unwrap();

        let link = generator.link_path(&ctx, &p.name);
        fs::remove_file(&link).unwrap();

        let status = generator.health_check(&ctx);
        assert!(
            matches!(status, HealthStatus::Error { ref message, .. } if message.contains("Theme link missing")),
            "Expected link missing error, got {:?}",
            status
        );
    }

    #[test]
    fn should_apply_theme_and_patch_tmux_conf() {
        let (tmp_dir, ctx) = create_test_context();
        let generator = TmuxGenerator;
        let p = Palette::mock();
        let root = tmp_dir.path();

        let tmux_dir = root.join(".config").join("tmux");
        let tmux_conf = tmux_dir.join("tmux.conf");
        fs::create_dir_all(&tmux_dir).unwrap();
        fs::write(&tmux_conf, "run '~/.tmux/plugins/tpm/tpm'").unwrap();

        generator.apply(&p, &ctx).expect("Apply failed");
        let content = fs::read_to_string(&tmux_conf).expect("Read failed");

        assert!(
            content.contains("# iris-theme"),
            "Marker missing. Content: \n{}\nPath: {:?}",
            content,
            tmux_conf
        );
        assert!(content.contains(&p.name));
    }

    #[test]
    fn should_fix_inject_before_tpm_issue_for_tmux() {
        let (_, ctx) = create_test_context();
        let generator = TmuxGenerator;
        let p = Palette::mock();

        let tmux_conf = get_tmux_conf_path(&ctx);
        fs::create_dir_all(tmux_conf.parent().unwrap()).unwrap();
        fs::write(&tmux_conf, "run '~/.tmux/plugins/tpm/tpm'").unwrap();

        let status = generator.health_check(&ctx);
        generator.fix(&status, &p, &ctx.silent()).unwrap();

        let content = fs::read_to_string(&tmux_conf).unwrap();
        let lines: Vec<&str> = content.lines().filter(|l| !l.is_empty()).collect();

        let theme_pos = lines
            .iter()
            .position(|l| l.contains("# iris-theme"))
            .expect("No theme line");
        let tpm_pos = lines
            .iter()
            .position(|l| l.contains("tpm"))
            .expect("No tpm line");

        assert!(theme_pos < tpm_pos, "Theme should be before TPM");
    }

    #[test]
    fn should_fix_wrong_theme_issue_for_tmux() {
        let (tmp_dir, mut ctx) = create_test_context();
        let generator = TmuxGenerator;
        let p = Palette::mock();
        let root = tmp_dir.path();

        ctx.state.current_theme = p.name.clone();

        let tmux_dir = root.join(".config").join("tmux");
        let tmux_conf = tmux_dir.join("tmux.conf");
        fs::create_dir_all(&tmux_dir).unwrap();
        fs::write(
            &tmux_conf,
            "source-file \"~/.config/tmux/themes/wrong.conf\" # iris-theme",
        )
        .unwrap();

        let status = generator.health_check(&ctx);

        assert!(
            matches!(status, HealthStatus::Warning(_)),
            "Expected Warning, got {:?}",
            status
        );

        generator.fix(&status, &p, &ctx.silent()).unwrap();

        let content = fs::read_to_string(&tmux_conf).unwrap();
        assert!(content.contains(&p.name));
        assert!(!content.contains("wrong.conf"));
    }

    #[test]
    fn should_fix_broken_symlink_for_tmux() {
        let (_, ctx) = create_test_context();
        let generator = TmuxGenerator;
        let p = Palette::mock();

        generator.apply(&p, &ctx).unwrap();
        let link_path = generator.link_path(&ctx, &p.name);

        fs::remove_file(&link_path).unwrap();

        let status = generator.health_check(&ctx);
        assert!(matches!(status, HealthStatus::Error { .. }));

        generator.fix(&status, &p, &ctx.silent()).unwrap();
        assert!(link_path.exists());
    }
}
