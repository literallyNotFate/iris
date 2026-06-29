use crate::{
    core::{IrisPaths, Templater},
    guards::FsRollbackGuard,
    log::Activity,
    models::{HealthStatus, Theme},
    modules::{Generator, GeneratorType},
    utils::{self},
};
use anyhow::{Context, Result};
use colored::Colorize;
use std::{fs, path::PathBuf};

/// Config generator for fzf utility
pub struct FzfGenerator;

impl Generator for FzfGenerator {
    fn name(&self) -> &str {
        "fzf"
    }

    fn generator_type(&self) -> GeneratorType {
        GeneratorType::Tool
    }

    fn target_file_name(&self, _theme: &str) -> String {
        "fzf.sh".into()
    }

    fn cache_path(&self, paths: &IrisPaths, _theme: &str) -> PathBuf {
        paths.bin.join(self.target_file_name(""))
    }

    fn link_path(&self, paths: &IrisPaths, _theme: &str) -> PathBuf {
        paths
            .config
            .parent()
            .and_then(|p| p.parent())
            .unwrap_or(&paths.config)
            .join(".zshrc")
    }

    fn apply(
        &self,
        theme: &Theme,
        paths: &IrisPaths,
        templater: &Templater,
        task: &mut Activity,
    ) -> Result<()> {
        let cache_file: PathBuf = self.cache_path(paths, "");
        let backup_path: PathBuf = cache_file.with_extension("sh.bak");

        task.info(&format!(
            "Generating {} script in: {}",
            self.name().bold().cyan(),
            utils::pretty_path(&cache_file).magenta()
        ));

        let rollback_guard = FsRollbackGuard::new(cache_file.clone(), backup_path);

        self.ensure_cache_file(theme, paths, templater)?;
        rollback_guard.commit();

        task.info(&format!(
            "{} theme applied to {}",
            theme.name.yellow(),
            self.name().bold().cyan()
        ));
        Ok(())
    }

    fn build_render_context(&self, theme: &Theme) -> tera::Context {
        let mut c = tera::Context::new();
        let strip = |hex: &str| hex.trim_start_matches('#').to_string();

        c.insert("theme_name", &theme.name);
        c.insert("fg", &strip(&theme.colors.fg));
        c.insert("bg", &strip(&theme.colors.bg));
        c.insert("accent", &strip(&theme.colors.ansi[3]));
        c.insert("match_c", &strip(&theme.colors.ansi[5]));
        c.insert("dimmed", &strip(&theme.colors.ansi[8]));

        c
    }

    fn health_check(&self, paths: &IrisPaths, _theme: &str) -> HealthStatus {
        if !self.is_installed() {
            return HealthStatus::Warning("`fzf` binary not found".into());
        }

        let zshrc: PathBuf = self.link_path(paths, "");
        let cache_file: PathBuf = self.cache_path(paths, "");
        let zshrc_status = HealthStatus::check_file(&zshrc, ".zshrc");

        if zshrc_status.is_error() {
            return HealthStatus::error(
                ".zshrc not found",
                Some("`fzf` theme requires a shell config to source the colors"),
            );
        }

        let content: String = fs::read_to_string(&zshrc).unwrap_or_default();
        if !content.contains("fzf.sh") {
            return HealthStatus::error(
                "fzf.sh is not sourced in .zshrc",
                Some(format!(
                    "Add 'source \"{}\"' to your .zshrc",
                    cache_file.display()
                )),
            );
        }

        let cache_status = HealthStatus::check_file(&cache_file, "`fzf` theme file");
        if cache_status.is_error() {
            return HealthStatus::error(
                "`fzf` theme file missing from cache",
                Some("Run `iris sync` or `iris health --fix` to regenerate it"),
            );
        }

        HealthStatus::Ok
    }

    fn fix(
        &self,
        status: &HealthStatus,
        theme: &Theme,
        paths: &IrisPaths,
        templater: &Templater,
        task: &mut Activity,
    ) -> Result<()> {
        if !status.is_error() && !status.is_warning() {
            return task.log.action(
                &format!("Re-applied `{}` configuration", self.name().bold()),
                || self.apply(theme, paths, templater, &mut task.muted()),
            );
        }

        let mut fixed = false;
        if status.contains("missing from cache") {
            task.log.action("Restoring missing theme file", || {
                self.apply(theme, paths, templater, &mut task.muted())
            })?;
            fixed = true;
        }

        if status.contains("not sourced") {
            let zshrc = self.link_path(paths, "");
            let backup = zshrc.with_extension("zshrc.bak");
            let rollback_guard = FsRollbackGuard::new(zshrc.clone(), backup);

            task.log.action(
                &format!("Injected source line into {}", ".zshrc".magenta()),
                || self.inject_source_line(paths),
            )?;

            rollback_guard.commit();
            fixed = true;
        }

        if !fixed {
            task.log
                .action("Regenerating complete `fzf` configuration", || {
                    self.apply(theme, paths, templater, &mut task.muted())
                })?;
        }

        Ok(())
    }

    fn clear(&self, paths: &IrisPaths) -> Result<()> {
        let zshrc: PathBuf = self.link_path(paths, "");
        if zshrc.exists() {
            let content: String = fs::read_to_string(&zshrc)?;
            let clean_content: String = self.remove_iris_lines(&content);

            if content != clean_content {
                let backup = zshrc.with_extension("zshrc.bak");
                let rollback_guard = FsRollbackGuard::new(zshrc.clone(), backup);

                fs::write(&zshrc, clean_content.trim())?;
                rollback_guard.commit();
            }
        }

        let cache_file: PathBuf = self.cache_path(paths, "");
        if cache_file.exists() {
            fs::remove_file(&cache_file).with_context(|| {
                format!("Failed to remove fzf cache file: {}", cache_file.display())
            })?;
        }
        Ok(())
    }

    fn remove_theme(&self, paths: &IrisPaths, theme_name: &str) -> Result<()> {
        let theme_name_lower: String = theme_name.to_lowercase();
        let zshrc: PathBuf = self.link_path(paths, "");

        if zshrc.exists() {
            let content: String = fs::read_to_string(&zshrc)?;
            if content.contains(&theme_name_lower) && content.contains("# iris:fzf") {
                self.clear(paths)?;
            }
        }

        let cache_file: PathBuf = self.cache_path(paths, &theme_name_lower);
        if cache_file.exists() {
            fs::remove_file(cache_file)?;
        }

        Ok(())
    }
}

impl FzfGenerator {
    fn remove_iris_lines(&self, content: &str) -> String {
        content
            .lines()
            .filter(|line| !line.contains("# iris:fzf"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn ensure_cache_file(
        &self,
        theme: &Theme,
        paths: &IrisPaths,
        templater: &Templater,
    ) -> Result<PathBuf> {
        let cache_file: PathBuf = self.cache_path(paths, &theme.name);
        let render_ctx = self.build_render_context(theme);
        let content: String = templater.render(&self.template_path(), &render_ctx)?;

        if let Some(parent) = cache_file.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create fzf cache directory: {}", parent.display())
            })?;
        }

        fs::write(&cache_file, content)
            .with_context(|| format!("Failed to write fzf cache: {}", cache_file.display()))?;

        Ok(cache_file)
    }

    fn inject_source_line(&self, paths: &IrisPaths) -> Result<()> {
        let zshrc: PathBuf = self.link_path(paths, "");
        let cache_file: PathBuf = self.cache_path(paths, "");
        let source_line: String = format!(
            "[ -f \"{0}\" ] && source \"{0}\" # iris:fzf",
            cache_file.display()
        );

        let content: String = fs::read_to_string(&zshrc).unwrap_or_default();
        if content.contains("# iris:fzf") {
            return Ok(());
        }

        let mut new_content: String = content.trim_end().to_string();
        if !new_content.is_empty() {
            new_content.push_str("\n\n");
        }

        new_content.push_str("# Import fzf theme from iris\n");
        new_content.push_str(&source_line);
        new_content.push('\n');

        fs::write(&zshrc, new_content)
            .with_context(|| format!("Failed to update configuration file: {}", zshrc.display()))?;

        Ok(())
    }
}

/// Tests for fzf generator
#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::tests::mock_context;

    /// Unit-tests for fzf
    mod unit {
        use super::*;

        #[test]
        fn should_return_fzf_metadata() {
            let generator = FzfGenerator;
            assert_eq!(generator.name(), "fzf");
            assert_eq!(generator.generator_type(), GeneratorType::Tool);
            assert_eq!(generator.target_file_name("any"), "fzf.sh");
        }

        #[test]
        fn should_build_valid_render_context() {
            let generator = FzfGenerator;
            let theme: Theme = Theme::mock();
            let render_ctx = generator.build_render_context(&theme);
            let fg = render_ctx.get("fg").unwrap().as_str().unwrap();

            assert!(!fg.starts_with('#'));
        }

        #[test]
        fn should_apply_fzf_theme_to_cache() {
            let (_, ctx) = mock_context();
            let generator = FzfGenerator;
            let theme: Theme = Theme::mock();

            let mut task = ctx.log.step("Test", false).muted();
            generator
                .apply(&theme, &ctx.paths, &ctx.templater, &mut task)
                .unwrap();

            let cache_file = ctx.paths.bin.join("fzf.sh");
            assert!(cache_file.exists(), "Cache file fzf.sh was not created");

            let content = fs::read_to_string(cache_file).unwrap();
            assert!(content.contains("export FZF_DEFAULT_OPTS="));
            assert!(content.contains(&theme.name));
        }

        #[test]
        fn should_clear_generated_files_for_fzf() {
            let (_, ctx) = mock_context();
            let generator = FzfGenerator;
            let theme: Theme = Theme::mock();

            let mut task = ctx.log.step("Test", false).muted();
            generator
                .apply(&theme, &ctx.paths, &ctx.templater, &mut task)
                .unwrap();

            let fzf_script = generator.cache_path(&ctx.paths, "");
            assert!(fzf_script.exists());

            generator.clear(&ctx.paths).unwrap();
            assert!(
                !fzf_script.exists(),
                "Clear should remove the generated fzf.sh script"
            );
        }

        #[test]
        fn should_remove_theme_for_fzf() {
            let (_, ctx) = mock_context();
            let generator = FzfGenerator;
            let theme: Theme = Theme::mock();

            let mut task = ctx.log.step("Test", false).muted();
            generator
                .apply(&theme, &ctx.paths, &ctx.templater, &mut task)
                .unwrap();

            let fzf_script = generator.cache_path(&ctx.paths, "");
            assert!(fzf_script.exists());

            generator.remove_theme(&ctx.paths, &theme.name).unwrap();
            assert!(
                !fzf_script.exists(),
                "remove_theme should delete the fzf.sh script"
            );
        }
    }

    /// Integration tests for yazi
    mod integration {
        use super::*;
        use crate::skip_if_not_installed;

        #[test]
        fn should_return_health_ok_for_fzf() {
            skip_if_not_installed!(FzfGenerator);

            let (_, ctx) = mock_context();
            let generator = FzfGenerator;
            let theme: Theme = Theme::mock();

            let mut task = ctx.log.step("Test", true).muted();
            generator
                .apply(&theme, &ctx.paths, &ctx.templater, &mut task)
                .unwrap();

            let zshrc_path = generator.link_path(&ctx.paths, "");
            let cache_file = generator.cache_path(&ctx.paths, "any");

            fs::create_dir_all(zshrc_path.parent().unwrap()).unwrap();
            fs::write(&zshrc_path, format!("source \"{}\"", cache_file.display())).unwrap();

            let status = generator.health_check(&ctx.paths, &theme.name);
            assert!(status.is_ok(), "Expected Ok, got: {status}");
        }

        #[test]
        fn should_return_health_error_no_zshrc_for_fzf() {
            skip_if_not_installed!(FzfGenerator);

            let (_, ctx) = mock_context();
            let generator = FzfGenerator;
            let status = generator.health_check(&ctx.paths, &ctx.state.theme.current_theme);

            assert!(
                status.is_error(),
                "Expected Error due to missing .zshrc, got: {status}"
            );
            assert!(status.contains(".zshrc not found"));
        }

        #[test]
        fn should_return_health_error_not_sourced_for_fzf() {
            skip_if_not_installed!(FzfGenerator);

            let (tmp_dir, ctx) = mock_context();
            let generator = FzfGenerator;
            let root = tmp_dir.path();
            let zshrc_path = root.join(".zshrc");

            fs::write(&zshrc_path, "alias ls='ls --color=auto'").unwrap();

            let status = generator.health_check(&ctx.paths, &ctx.state.theme.current_theme);

            assert!(
                status.is_error(),
                "Expected Error because fzf.sh is not in .zshrc, got: {status}"
            );
            assert!(status.contains("not sourced"));
        }

        #[test]
        fn should_return_health_error_cache_file_missing_for_fzf() {
            skip_if_not_installed!(FzfGenerator);

            let (_tmp_dir, ctx) = mock_context();
            let generator = FzfGenerator;

            let zshrc = generator.link_path(&ctx.paths, "");
            let cache_file = generator.cache_path(&ctx.paths, "");

            fs::create_dir_all(zshrc.parent().unwrap()).unwrap();
            fs::write(&zshrc, format!("source {:?} # iris:fzf", cache_file)).unwrap();

            if cache_file.exists() {
                fs::remove_file(&cache_file).unwrap();
            }

            let status = generator.health_check(&ctx.paths, &ctx.state.theme.current_theme);

            assert!(status.is_error(), "Expected Error, got: {status}");
            assert!(status.contains("missing from cache") || status.contains("not found"));
        }

        #[test]
        fn should_fix_source_line_injection_for_fzf() {
            skip_if_not_installed!(FzfGenerator);

            let (tmp_dir, ctx) = mock_context();
            let generator = FzfGenerator;
            let theme: Theme = Theme::mock();
            let root = tmp_dir.path();

            let zshrc = root.join(".zshrc");
            fs::write(&zshrc, "# Initial zshrc\n").unwrap();

            let mut task = ctx.log.step("Test", false).muted();
            generator
                .apply(&theme, &ctx.paths, &ctx.templater, &mut task)
                .unwrap();

            let status = generator.health_check(&ctx.paths, &theme.name);
            assert!(status.is_error());
            assert!(status.contains("not sourced"));

            generator
                .fix(&status, &theme, &ctx.paths, &ctx.templater, &mut task)
                .expect("Fix failed");

            let updated_content = fs::read_to_string(&zshrc).unwrap();
            let cache_file = generator.cache_path(&ctx.paths, &theme.name);

            assert!(
                updated_content.contains(&cache_file.to_str().unwrap()),
                "zshrc should now contain the source line for fzf.sh"
            );
            assert!(updated_content.contains("# iris:fzf"));

            let final_status = generator.health_check(&ctx.paths, &theme.name);
            assert!(
                final_status.is_ok(),
                "Final status should be Ok, got: {final_status}"
            );
        }

        #[test]
        fn should_fix_missing_cache_for_fzf() {
            skip_if_not_installed!(FzfGenerator);

            let (_, ctx) = mock_context();
            let generator = FzfGenerator;
            let theme: Theme = Theme::mock();

            let zshrc = generator.link_path(&ctx.paths, "");
            let cache_file = generator.cache_path(&ctx.paths, &theme.name);

            fs::create_dir_all(zshrc.parent().unwrap()).unwrap();
            fs::write(
                &zshrc,
                format!("source \"{}\" # fzf.sh", cache_file.display()),
            )
            .unwrap();

            let status = HealthStatus::error("missing from cache", None::<String>);

            let mut task = ctx.log.step("Test", false).muted();
            generator
                .fix(&status, &theme, &ctx.paths, &ctx.templater, &mut task)
                .expect("Fix failed");

            assert!(cache_file.exists(), "Cache file should be recreated");

            let content = fs::read_to_string(cache_file).unwrap();
            assert!(content.contains("export FZF_DEFAULT_OPTS="));
        }
    }
}
