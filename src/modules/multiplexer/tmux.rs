use crate::{
    core::{IrisPaths, Templater},
    guards::FsRollbackGuard,
    log::Task,
    models::{HealthStatus, Theme},
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
        task: &mut Task,
    ) -> Result<()> {
        task.info(&format!(
            "Generating {} theme for {}",
            theme.name.yellow(),
            self.name().bold().cyan(),
        ));

        let cache_file: PathBuf = self.ensure_cache_file(theme, paths, templater)?;
        let link_path: PathBuf = self.link_path(paths, &theme.name.to_lowercase());
        let backup_path: PathBuf = link_path.with_extension("bak");

        task.info(&format!(
            "Linking {} theme to {}...",
            self.name().bold(),
            utils::pretty_path(&link_path).magenta(),
        ));

        let rollback_guard = FsRollbackGuard::new(link_path.clone(), backup_path);

        self.ensure_symlink(&cache_file, &link_path)?;
        let conf_path = self.resolve_tmux_conf_path(paths);

        if conf_path.exists() {
            task.info(&format!(
                "Patching tmux.conf to source {}...",
                theme.name.yellow(),
            ));

            let conf_backup: PathBuf = conf_path.with_extension("bak");
            let conf_guard = FsRollbackGuard::new(conf_path.clone(), conf_backup);

            self.update_tmux_conf(&conf_path, &theme.name.to_lowercase())?;
            conf_guard.commit();
        }

        rollback_guard.commit();
        Ok(())
    }

    fn build_render_context(&self, theme: &Theme) -> tera::Context {
        let mut c = tera::Context::new();

        c.insert("theme_name", &theme.name);
        c.insert("bg", &theme.colors.bg);
        c.insert("fg", &theme.colors.fg);
        c.insert("keyword", &theme.colors.keyword);
        c.insert("comment", &theme.colors.comment);
        c.insert("operator", &theme.colors.operator);
        c.insert("gutter_fg", &theme.colors.gutter_fg);
        c.insert("line_hl", &theme.colors.line_hl);
        c.insert("func", &theme.colors.func);
        c.insert("tag", &theme.colors.tag);

        c.insert("green", &theme.colors.ansi[10]);
        c.insert("yellow", &theme.colors.ansi[3]);
        c.insert("blue", &theme.colors.ansi[12]);

        c
    }

    fn health_check(&self, paths: &IrisPaths, theme: &str) -> HealthStatus {
        if !self.is_installed() {
            return HealthStatus::Warning("`tmux` binary not found".into());
        }

        let tmux_conf: PathBuf = self.resolve_tmux_conf_path(paths);
        let config_status = HealthStatus::check_file(&tmux_conf, "tmux.conf");

        if config_status.is_error() {
            return HealthStatus::error(
                "tmux.conf missing",
                Some("Create ~/.config/tmux/tmux.conf"),
            );
        }

        if !theme.is_empty() {
            let content: String = fs::read_to_string(&tmux_conf).unwrap_or_default();
            let marker: &str = "# iris-theme";
            if !content.contains(marker) {
                return HealthStatus::Warning(format!(
                    "Theme is not sourced in {}. Run `iris sync` or `iris health --fix` to source.",
                    tmux_conf.display()
                ));
            }

            let expected_file: String = format!("{}.conf", theme);
            if !content.contains(&expected_file) {
                return HealthStatus::Warning(format!(
                    "tmux.conf sources a different theme, not `{theme}`"
                ));
            }

            let link: PathBuf = self.link_path(paths, theme);
            let link_status = HealthStatus::check_symlink(&link, "Theme link");
            if link_status.is_error() {
                return HealthStatus::error(
                    format!("Theme link missing: {}", link.display()),
                    Some("Run `iris health --fix` to restore symlink"),
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
        task: &mut Task,
    ) -> Result<()> {
        if !status.is_error() && !status.is_warning() {
            return task.log.action(
                &format!("Re-applied `{}` configuration", self.name().bold()),
                || self.apply(theme, paths, templater, &mut task.muted()),
            );
        }

        let mut fixed = false;
        if status.contains("Theme link missing") {
            task.log
                .action("Regenerated `tmux` theme file and symlink", || {
                    self.apply(theme, paths, templater, &mut task.muted())
                })?;
            fixed = true;
        }

        if (status.contains("not sourced") || status.contains("different theme")) && !fixed {
            let conf_path: PathBuf = self.resolve_tmux_conf_path(paths);
            let config_backup: PathBuf = conf_path.with_extension("bak");

            let rollback_guard = FsRollbackGuard::new(conf_path.clone(), config_backup);

            task.log.action("Repaired tmux.conf source line", || {
                self.update_tmux_conf(&conf_path, &theme.name.to_lowercase())
            })?;

            rollback_guard.commit();
            fixed = true;
        }

        if !fixed {
            task.log
                .action("Regenerated complete `tmux` configuration", || {
                    self.apply(theme, paths, templater, &mut task.muted())
                })?;
        }

        Ok(())
    }
}

impl TmuxGenerator {
    /// Ensure tmux.conf sources the iris theme file.
    /// Replaces an existing Iris source line or appends one before the `run` line (tpm).
    fn update_tmux_conf(&self, path: &Path, theme_name: &str) -> Result<()> {
        if !path.exists() {
            return Ok(());
        }

        let source_line = format!(
            "source-file \"~/.config/tmux/themes/{}.conf\" # iris-theme",
            theme_name
        );

        let content: String = fs::read_to_string(path)
            .with_context(|| format!("Failed to read `tmux` config: {}", path.display()))?;

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

        let new_content: String = lines.join("\n");
        fs::write(path, new_content)
            .with_context(|| format!("Failed to update tmux.conf: {}", path.display()))?;
        Ok(())
    }

    fn resolve_tmux_conf_path(&self, paths: &IrisPaths) -> PathBuf {
        let themes_dir: PathBuf = self.resolve_config_directory(paths);

        themes_dir
            .parent()
            .unwrap_or(&themes_dir)
            .to_path_buf()
            .join("tmux.conf")
    }

    fn ensure_cache_file(
        &self,
        theme: &Theme,
        paths: &IrisPaths,
        templater: &Templater,
    ) -> Result<PathBuf> {
        let cache_file: PathBuf = self.cache_path(paths, &theme.name.to_lowercase());
        let render_ctx = self.build_render_context(theme);
        let content: String = templater.render(&self.template_path(), &render_ctx)?;

        if let Some(parent) = cache_file.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create `tmux` directory: {}", parent.display())
            })?;
        }

        fs::write(&cache_file, content).with_context(|| {
            format!(
                "Failed to write `tmux` theme file: {}",
                cache_file.display()
            )
        })?;

        Ok(cache_file)
    }

    fn ensure_symlink(&self, target: &Path, link: &Path) -> Result<()> {
        if link.exists() || link.is_symlink() {
            fs::remove_file(link)
                .with_context(|| format!("Failed to remove `tmux` old link: {}", link.display()))?;
        }

        if let Some(parent) = link.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "Failed to create parent directory for `tmux` link: {}",
                    parent.display()
                )
            })?;
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            symlink(target, link).with_context(|| {
                format!(
                    "Failed to create `tmux` symlink: {} -> {}",
                    target.display(),
                    link.display()
                )
            })?;
        }
        Ok(())
    }
}

/// Unit-tests for tmux generator
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{IrisContext, tests::create_test_context};
    use tempdir::TempDir;

    // Helper function to get tmux conf just like in generator
    fn get_tmux_conf_path(ctx: &IrisContext) -> PathBuf {
        let generator = TmuxGenerator;
        let themes_dir = generator.resolve_config_directory(&ctx.paths);
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
        let theme: Theme = Theme::mock();
        let ctx = generator.build_render_context(&theme);

        assert_eq!(ctx.get("bg").unwrap().as_str().unwrap(), theme.colors.bg);
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
        let theme: Theme = Theme::mock();
        let root = tmp_dir.path();

        let tmux_dir = root.join(".config").join("tmux");
        let themes_dir = tmux_dir.join("themes");
        let tmux_conf = tmux_dir.join("tmux.conf");

        fs::create_dir_all(&themes_dir).unwrap();
        fs::write(
            &tmux_conf,
            format!(
                "source-file \"~/.config/tmux/themes/{}.conf\" # iris-theme",
                theme.name
            ),
        )
        .unwrap();

        let mut task = ctx.log.step("Test", false).as_quiet();
        ctx.state.current_theme = theme.name.clone();
        generator
            .apply(&theme, &ctx.paths, &ctx.templater, &mut task)
            .expect("Apply failed");

        let status = generator.health_check(&ctx.paths, &theme.name);
        assert!(status.is_ok(), "Expected Ok, got: {status}");
    }

    #[test]
    fn should_return_health_warning_missing_marker_for_tmux() {
        let (tmp_dir, mut ctx) = create_test_context();
        let generator = TmuxGenerator;
        let theme: Theme = Theme::mock();
        let root = tmp_dir.path();

        let tmux_dir = root.join(".config").join("tmux");
        fs::create_dir_all(&tmux_dir).unwrap();

        let tmux_conf = tmux_dir.join("tmux.conf");
        fs::write(
            &tmux_conf,
            format!(
                "source-file \"~/.config/tmux/themes/{}.conf\" # iris-theme",
                theme.name
            ),
        )
        .unwrap();

        let mut task = ctx.log.step("Test", false).as_quiet();
        ctx.state.current_theme = theme.name.clone();
        generator
            .apply(&theme, &ctx.paths, &ctx.templater, &mut task)
            .unwrap();

        let link = generator.link_path(&ctx.paths, &theme.name);
        if link.exists() {
            fs::remove_file(&link).unwrap();
        }

        let status = generator.health_check(&ctx.paths, &theme.name);
        assert!(
            status.is_error(),
            "Expected Error due to missing link, got: {status}"
        );
        assert!(status.contains("Theme link missing"));
    }

    #[test]
    fn should_return_health_warning_wrong_theme_sourced_for_tmux() {
        let (tmp_dir, mut ctx) = create_test_context();
        let generator = TmuxGenerator;
        let theme: Theme = Theme::mock();
        let root = tmp_dir.path();

        let tmux_dir = root.join(".config").join("tmux");
        fs::create_dir_all(&tmux_dir).unwrap();
        let tmux_conf = tmux_dir.join("tmux.conf");

        ctx.state.current_theme = theme.name.clone();
        fs::write(
            &tmux_conf,
            "source-file ~/.config/tmux/themes/wrong.conf # iris-theme",
        )
        .unwrap();

        let status = generator.health_check(&ctx.paths, &theme.name);
        assert!(
            status.is_warning(),
            "Expected Warning for wrong theme file, got: {status}"
        );
        assert!(status.contains("sources a different theme"));
    }

    #[test]
    fn should_return_health_error_link_missing_for_tmux() {
        let (tmp_dir, mut ctx) = create_test_context();
        let generator = TmuxGenerator;
        let theme: Theme = Theme::mock();
        let root = tmp_dir.path();

        let tmux_dir = root.join(".config").join("tmux");
        let tmux_conf = tmux_dir.join("tmux.conf");
        fs::create_dir_all(&tmux_dir).unwrap();
        fs::write(
            &tmux_conf,
            format!(
                "source-file \"~/.config/tmux/themes/{}.conf\" # iris-theme",
                theme.name
            ),
        )
        .unwrap();

        let mut task = ctx.log.step("Test", false).as_quiet();
        ctx.state.current_theme = theme.name.clone();
        generator
            .apply(&theme, &ctx.paths, &ctx.templater, &mut task)
            .unwrap();

        let link = generator.link_path(&ctx.paths, &theme.name);
        fs::remove_file(&link).unwrap();

        let status = generator.health_check(&ctx.paths, &theme.name);
        assert!(
            status.is_error(),
            "Expected Error for deleted symlink, got: {status}"
        );
        assert!(status.contains("Theme link missing"));
    }

    #[test]
    fn should_apply_theme_and_patch_tmux_conf() {
        let (tmp_dir, ctx) = create_test_context();
        let generator = TmuxGenerator;
        let theme: Theme = Theme::mock();
        let root = tmp_dir.path();

        let tmux_dir = root.join(".config").join("tmux");
        let tmux_conf = tmux_dir.join("tmux.conf");
        fs::create_dir_all(&tmux_dir).unwrap();
        fs::write(&tmux_conf, "run '~/.tmux/plugins/tpm/tpm'").unwrap();

        let mut task = ctx.log.step("Test", false).as_quiet();
        generator
            .apply(&theme, &ctx.paths, &ctx.templater, &mut task)
            .expect("Apply failed");
        let content = fs::read_to_string(&tmux_conf).expect("Read failed");

        assert!(
            content.contains("# iris-theme"),
            "Marker missing in tmux.conf"
        );
        assert!(content.contains(&theme.name));
    }

    #[test]
    fn should_fix_inject_before_tpm_issue_for_tmux() {
        let (_, ctx) = create_test_context();
        let generator = TmuxGenerator;
        let theme: Theme = Theme::mock();

        let tmux_conf = get_tmux_conf_path(&ctx);
        fs::create_dir_all(tmux_conf.parent().unwrap()).unwrap();
        fs::write(&tmux_conf, "run '~/.tmux/plugins/tpm/tpm'").unwrap();

        let mut task = ctx.log.step("Test", false).as_quiet();
        let status = generator.health_check(&ctx.paths, &theme.name);
        generator
            .fix(&status, &theme, &ctx.paths, &ctx.templater, &mut task)
            .expect("Fix failed");

        let content = fs::read_to_string(&tmux_conf).unwrap();
        let lines: Vec<&str> = content.lines().filter(|l| !l.is_empty()).collect();

        let theme_pos = lines
            .iter()
            .position(|l| l.contains("# iris-theme"))
            .expect("No theme line injected");
        let tpm_pos = lines
            .iter()
            .position(|l| l.contains("tpm"))
            .expect("No tpm line found");

        assert!(theme_pos < tpm_pos, "Theme should be before TPM");
    }

    #[test]
    fn should_fix_wrong_theme_issue_for_tmux() {
        let (_, mut ctx) = create_test_context();
        let generator = TmuxGenerator;
        let theme: Theme = Theme::mock();
        ctx.state.current_theme = theme.name.clone();

        let mut task = ctx.log.step("Test", false).as_quiet();
        generator
            .apply(&theme, &ctx.paths, &ctx.templater, &mut task)
            .unwrap();

        let tmux_conf = generator.resolve_tmux_conf_path(&ctx.paths);
        fs::write(&tmux_conf, "source-file wrong_theme.conf").unwrap();

        let status = generator.health_check(&ctx.paths, &theme.name);
        assert!(
            status.is_warning() || status.is_error(),
            "Expected Warning/Error for wrong theme, got: {status}"
        );

        generator
            .fix(&status, &theme, &ctx.paths, &ctx.templater, &mut task)
            .expect("Fix failed");
        assert!(generator.health_check(&ctx.paths, &theme.name).is_ok());
    }

    #[test]
    fn should_fix_broken_symlink_for_tmux() {
        let (_, mut ctx) = create_test_context();
        let generator = TmuxGenerator;
        let theme: Theme = Theme::mock();
        ctx.state.current_theme = theme.name.clone();

        let mut task = ctx.log.step("Test", false).as_quiet();
        generator
            .apply(&theme, &ctx.paths, &ctx.templater, &mut task)
            .unwrap();
        let tmux_conf = generator.resolve_tmux_conf_path(&ctx.paths);
        if let Some(parent) = tmux_conf.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mock_content = format!("# iris-theme\nsource-file {}.conf", theme.name);
        fs::write(&tmux_conf, mock_content).unwrap();

        let link_path = generator.link_path(&ctx.paths, &theme.name);
        if link_path.exists() {
            fs::remove_file(&link_path).unwrap();
        }

        let status = generator.health_check(&ctx.paths, &theme.name);
        assert!(
            status.is_error(),
            "Expected Error for broken link, got: {status}"
        );
        assert!(status.contains("Theme link missing"));

        generator
            .fix(&status, &theme, &ctx.paths, &ctx.templater, &mut task)
            .expect("Fix failed");
        assert!(link_path.exists(), "Symlink should be recreated after fix");
        assert!(generator.health_check(&ctx.paths, &theme.name).is_ok());
    }

    #[test]
    fn should_clear_generated_files_for_tmux() {
        let (_, ctx) = create_test_context();
        let generator = TmuxGenerator;

        let cache_dir = ctx.paths.generators.join(generator.name());
        fs::create_dir_all(&cache_dir).unwrap();

        let test_file = cache_dir.join("test.conf");
        fs::write(&test_file, "theme content").unwrap();

        assert!(
            cache_dir.exists(),
            "Cache directory should exist before clearing"
        );

        generator.clear(&ctx.paths).unwrap();

        assert!(
            !cache_dir.exists(),
            "Clear should remove the entire generator cache directory"
        );
    }

    #[test]
    fn should_remove_theme_for_tmux() {
        let (_, ctx) = create_test_context();
        let generator = TmuxGenerator;

        let cache_file = generator.cache_path(&ctx.paths, "test");
        let link_file = generator.theme_path(&ctx.paths, "test");

        fs::create_dir_all(cache_file.parent().unwrap()).unwrap();
        fs::create_dir_all(link_file.parent().unwrap()).unwrap();

        fs::write(&cache_file, "theme content").unwrap();
        fs::write(&link_file, "theme content").unwrap();

        assert!(
            cache_file.exists(),
            "Cache file should exist before removal"
        );
        assert!(link_file.exists(), "Theme file should exist before removal");

        generator.remove_theme(&ctx.paths, "test").unwrap();

        assert!(
            !cache_file.exists(),
            "remove_theme should delete the cache file"
        );
        assert!(
            !link_file.exists(),
            "remove_theme should delete the target theme file"
        );
    }
}
