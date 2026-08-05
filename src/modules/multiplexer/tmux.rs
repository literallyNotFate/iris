use crate::{
    core::{InjectionPosition, IrisEngine},
    infra::IrisPaths,
    models::{HealthStatus, Issue},
    modules::{Generator, GeneratorType, Strategy, traits::*},
};
use std::{fs, path::PathBuf};

/// Config generator for tmux
pub struct TmuxGenerator;

impl Identifiable for TmuxGenerator {
    fn name(&self) -> &str {
        "tmux"
    }

    fn generator_type(&self) -> GeneratorType {
        GeneratorType::Multiplexer
    }
}

impl PathResolvable for TmuxGenerator {
    fn target_file_name(&self, theme: &str) -> String {
        format!("{}.conf", theme)
    }

    fn link_path(&self, paths: &IrisPaths, theme: &str) -> PathBuf {
        self.resolve_config_directory(paths)
            .join("themes")
            .join(self.target_file_name(theme))
    }
}

impl Generator for TmuxGenerator {
    fn strategy(&self) -> Strategy {
        Strategy::Symlink
    }

    fn pre_apply(&self, engine: &IrisEngine) -> anyhow::Result<()> {
        let config_path: PathBuf = self
            .resolve_config_directory(engine.paths)
            .join("tmux.conf");
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        engine.remove_marker(&config_path, "source-file \"~/.config/tmux/themes/")?;
        let import: String = format!(
            "source-file \"~/.config/tmux/themes/{}.conf\"",
            engine.theme.name.to_lowercase()
        );
        engine.inject_line(&config_path, &import, InjectionPosition::Start)
    }
}

impl Diagnosable for TmuxGenerator {
    fn health_check(&self, paths: &IrisPaths, theme: &str) -> HealthStatus {
        if !self.is_installed() {
            return HealthStatus::error(Issue::BinaryNotFound);
        }

        let conf_path: PathBuf = self.resolve_config_directory(paths).join("tmux.conf");
        let link_path: PathBuf = self.link_path(paths, theme);

        let config_status = HealthStatus::check_file(&conf_path, Issue::ConfigMissing);
        if !config_status.is_ok() {
            return config_status;
        }

        if !theme.is_empty() {
            let content: String = fs::read_to_string(&conf_path).unwrap_or_default();
            let expected_import: String = format!(
                "source-file \"~/.config/tmux/themes/{}.conf\"",
                theme.to_lowercase()
            );
            if !content.contains(&expected_import) {
                return HealthStatus::warn(Issue::ImportMissing);
            }

            let link_status = HealthStatus::check_symlink(&link_path, Issue::SymlinkInvalid);
            if !link_status.is_ok() {
                return link_status;
            }

            let expected_cache: PathBuf = self.cache_path(paths, theme);
            if let Ok(target) = fs::read_link(&link_path) {
                let abs_target: PathBuf = fs::canonicalize(&target).unwrap_or(target);
                let abs_expected = fs::canonicalize(&expected_cache).unwrap_or(expected_cache);

                if abs_target != abs_expected {
                    return HealthStatus::warn(Issue::CacheMismatch);
                }
            }
        }

        HealthStatus::Ok
    }
}

impl Cleanable for TmuxGenerator {
    fn cleanup(&self, paths: &IrisPaths) -> anyhow::Result<()> {
        default_cleanup(self, paths)
    }

    fn remove_theme(&self, paths: &IrisPaths, theme_name: &str) -> anyhow::Result<()> {
        default_remove(self, paths, theme_name)
    }
}

impl Diffable for TmuxGenerator {
    fn config_path(&self, paths: &IrisPaths) -> PathBuf {
        self.resolve_config_directory(paths).join("tmux.conf")
    }

    fn diff_style(&self) -> DiffStyle {
        DiffStyle::InjectTop {
            build_ideal_line: |theme| {
                let theme_name = if theme.is_empty() { "default" } else { theme };
                format!(
                    "source-file \"~/.config/tmux/themes/{}.conf\"",
                    theme_name.to_lowercase()
                )
            },
            line_filter: |line| line.contains("source-file") && line.contains("themes/"),
        }
    }
}

/// Tests for tmux generator
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{core::IrisContext, models::Theme};

    /// Unit-tests for tmux
    mod unit {
        use super::*;

        #[test]
        fn should_return_metadata_for_tmux() {
            let generator = TmuxGenerator;
            assert_eq!(generator.name(), "tmux");
            assert_eq!(generator.generator_type(), GeneratorType::Multiplexer);
            assert_eq!(generator.target_file_name("dracula"), "dracula.conf");
        }

        #[test]
        fn should_build_valid_render_context_for_tmux() {
            let generator = TmuxGenerator;
            let (_, ctx) = IrisContext::mock();
            let theme: Theme = Theme::mock();
            let ctx = ctx.engine(&theme).build_context(&generator).unwrap();

            assert_eq!(ctx.get("bg").unwrap().as_str().unwrap(), theme.colors.bg);
            assert_eq!(ctx.get("fg").unwrap().as_str().unwrap(), theme.colors.fg);
            assert!(ctx.get("ansi").unwrap().is_array());
        }

        #[test]
        fn should_apply_theme_for_tmux() {
            let (tmp_dir, ctx) = IrisContext::with_templates(vec![(
                "multiplexer/tmux",
                "set -g status-style \"bg={{ bg }},fg={{ fg }}\"",
            )]);
            let generator = TmuxGenerator;
            let theme: Theme = Theme::mock();
            let root = tmp_dir.path();

            let tmux_dir = root.join(".config").join("tmux");
            let tmux_conf = tmux_dir.join("tmux.conf");
            fs::create_dir_all(&tmux_dir).unwrap();
            fs::write(&tmux_conf, "run '~/.tmux/plugins/tpm/tpm'").unwrap();

            let mut activity = ctx.log.step("Test", false).muted();
            ctx.engine(&theme)
                .execute_apply(&generator, &mut activity)
                .unwrap();
            let content = fs::read_to_string(&tmux_conf).expect("Read failed");

            assert!(
                content.contains("themes/"),
                "Theme path missing in tmux.conf"
            );
            assert!(content.contains(&theme.name.to_lowercase()));
        }

        #[test]
        fn should_clear_generated_files_for_tmux() {
            let (_, ctx) = IrisContext::mock();
            let generator = TmuxGenerator;

            let cache_dir = ctx.paths.generators.join(generator.name());
            fs::create_dir_all(&cache_dir).unwrap();

            let test_file = cache_dir.join("test.conf");
            fs::write(&test_file, "theme content").unwrap();

            assert!(
                cache_dir.exists(),
                "Cache directory should exist before clearing"
            );

            generator.cleanup(&ctx.paths).unwrap();

            assert!(
                !cache_dir.exists(),
                "Clear should remove the entire generator cache directory"
            );
        }

        #[test]
        fn should_remove_theme_for_tmux() {
            let (_, ctx) = IrisContext::mock();
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

    /// Integration tests for tmux
    mod integration {
        use super::*;
        use crate::skip_if_not_installed;

        #[test]
        fn should_return_health_ok_for_tmux() {
            skip_if_not_installed!(TmuxGenerator);

            let (tmp_dir, mut ctx) = IrisContext::with_templates(vec![(
                "multiplexer/tmux",
                "set -g status-style \"bg={{ bg }},fg={{ fg }}\"",
            )]);
            let generator = TmuxGenerator;
            let theme: Theme = Theme::mock();
            let root = tmp_dir.path();

            let tmux_dir = root.join(".config").join("tmux");
            let themes_dir = tmux_dir.join("themes");
            let tmux_conf = tmux_dir.join("tmux.conf");

            fs::create_dir_all(&themes_dir).unwrap();
            fs::write(
                &tmux_conf,
                format!("source-file \"~/.config/tmux/themes/{}.conf\"", theme.name),
            )
            .unwrap();

            let mut activity = ctx.log.step("Test", false).muted();
            ctx.state.theme.current_theme = theme.name.clone();
            ctx.engine(&theme)
                .execute_apply(&generator, &mut activity)
                .unwrap();

            let status = generator.health_check(&ctx.paths, &theme.name);
            assert!(status.is_ok(), "Expected Ok, got: {status}");
        }

        #[test]
        fn should_return_health_warning_missing_import_for_tmux() {
            skip_if_not_installed!(TmuxGenerator);

            let (tmp_dir, mut ctx) = IrisContext::with_templates(vec![(
                "multiplexer/tmux",
                "set -g status-style \"bg={{ bg }},fg={{ fg }}\"",
            )]);
            let generator = TmuxGenerator;
            let theme: Theme = Theme::mock();
            let root = tmp_dir.path();

            let tmux_dir = root.join(".config").join("tmux");
            fs::create_dir_all(&tmux_dir).unwrap();

            let tmux_conf = tmux_dir.join("tmux.conf");
            fs::write(&tmux_conf, "set -g mouse on").unwrap();
            ctx.state.theme.current_theme = theme.name.clone();

            let status = generator.health_check(&ctx.paths, &theme.name);
            assert!(status.is_warning(), "Expected Warning, got: {status}");
            assert!(status.contains("Theme not imported"));
        }

        #[test]
        fn should_return_health_warning_wrong_theme_sourced_for_tmux() {
            skip_if_not_installed!(TmuxGenerator);

            let (tmp_dir, mut ctx) = IrisContext::mock();
            let generator = TmuxGenerator;
            let theme: Theme = Theme::mock();
            let root = tmp_dir.path();

            let tmux_dir = root.join(".config").join("tmux");
            fs::create_dir_all(&tmux_dir).unwrap();
            let tmux_conf = tmux_dir.join("tmux.conf");

            ctx.state.theme.current_theme = theme.name.clone();
            fs::write(&tmux_conf, "source-file ~/.config/tmux/themes/wrong.conf").unwrap();

            let status = generator.health_check(&ctx.paths, &theme.name);
            assert!(status.is_warning(), "Expected Warning, got: {status}");
            assert!(status.contains("Theme not imported"));
        }

        #[test]
        fn should_return_health_error_link_missing_for_tmux() {
            skip_if_not_installed!(TmuxGenerator);

            let (tmp_dir, mut ctx) = IrisContext::mock();
            let generator = TmuxGenerator;
            let theme: Theme = Theme::mock();
            let root = tmp_dir.path();

            let tmux_dir = root.join(".config").join("tmux");
            let tmux_conf = tmux_dir.join("tmux.conf");
            fs::create_dir_all(&tmux_dir).unwrap();
            fs::write(
                &tmux_conf,
                format!("source-file \"~/.config/tmux/themes/{}.conf\"", theme.name),
            )
            .unwrap();

            let mut activity = ctx.log.step("Test", false).muted();
            ctx.state.theme.current_theme = theme.name.clone();
            ctx.engine(&theme)
                .execute_apply(&generator, &mut activity)
                .unwrap();

            let link = generator.link_path(&ctx.paths, &theme.name);
            fs::remove_file(&link).unwrap();

            let status = generator.health_check(&ctx.paths, &theme.name);
            assert!(status.is_error(), "Expected Error, got: {status}");
            assert!(status.contains("Invalid symlink"));
        }

        #[test]
        fn should_fix_inject_at_start_for_tmux() {
            skip_if_not_installed!(TmuxGenerator);

            let (_, ctx) = IrisContext::mock();
            let generator = TmuxGenerator;
            let theme: Theme = Theme::mock();

            let tmux_conf = generator
                .resolve_config_directory(&ctx.paths)
                .join("tmux.conf");
            fs::create_dir_all(tmux_conf.parent().unwrap()).unwrap();
            fs::write(&tmux_conf, "run '~/.tmux/plugins/tpm/tpm'").unwrap();

            let mut activity = ctx.log.step("Test", false).muted();
            let status = generator.health_check(&ctx.paths, &theme.name);
            ctx.engine(&theme)
                .execute_fix(&generator, &status, &mut activity)
                .unwrap();

            let content = fs::read_to_string(&tmux_conf).unwrap();
            let lines: Vec<&str> = content.lines().filter(|l| !l.is_empty()).collect();

            let theme_pos = lines
                .iter()
                .position(|l| l.contains("themes/"))
                .expect("No theme line injected");
            let tpm_pos = lines
                .iter()
                .position(|l| l.contains("tpm"))
                .expect("No tpm line found");

            assert!(theme_pos < tpm_pos, "Theme should be before TPM");
        }

        #[test]
        fn should_fix_wrong_theme_issue_for_tmux() {
            skip_if_not_installed!(TmuxGenerator);

            let (_, mut ctx) = IrisContext::mock();
            let generator = TmuxGenerator;
            let theme: Theme = Theme::mock();
            ctx.state.theme.current_theme = theme.name.clone();

            let mut activity = ctx.log.step("Test", false).muted();
            let engine = ctx.engine(&theme);
            engine.execute_apply(&generator, &mut activity).unwrap();

            let tmux_conf = generator
                .resolve_config_directory(&ctx.paths)
                .join("tmux.conf");
            fs::write(
                &tmux_conf,
                "source-file \"~/.config/tmux/themes/wrong.conf\"",
            )
            .unwrap();

            let status = generator.health_check(&ctx.paths, &theme.name);
            assert!(
                status.is_warning() || status.is_error(),
                "Expected Warning/Error for wrong theme, got: {status}"
            );

            engine
                .execute_fix(&generator, &status, &mut activity)
                .unwrap();
            assert!(generator.health_check(&ctx.paths, &theme.name).is_ok());
        }

        #[test]
        fn should_fix_broken_symlink_for_tmux() {
            skip_if_not_installed!(TmuxGenerator);

            let (_, mut ctx) = IrisContext::mock();
            let generator = TmuxGenerator;
            let theme: Theme = Theme::mock();
            ctx.state.theme.current_theme = theme.name.clone();

            let mut activity = ctx.log.step("Test", false).muted();
            let engine = ctx.engine(&theme);
            engine.execute_apply(&generator, &mut activity).unwrap();

            let tmux_conf = generator
                .resolve_config_directory(&ctx.paths)
                .join("tmux.conf");
            if let Some(parent) = tmux_conf.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            let mock_content = format!(
                "source-file \"~/.config/tmux/themes/{}.conf\"",
                theme.name.to_lowercase()
            );
            fs::write(&tmux_conf, mock_content).unwrap();

            let link_path = generator.link_path(&ctx.paths, &theme.name);
            if link_path.exists() {
                fs::remove_file(&link_path).unwrap();
            }

            let status = generator.health_check(&ctx.paths, &theme.name);
            assert!(status.is_error(), "Expected Error , got: {status}");

            engine
                .execute_fix(&generator, &status, &mut activity)
                .unwrap();
            assert!(link_path.exists(), "Symlink should be recreated after fix");
            assert!(generator.health_check(&ctx.paths, &theme.name).is_ok());
        }
    }
}
