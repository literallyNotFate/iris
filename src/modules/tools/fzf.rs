use crate::{
    infra::IrisPaths,
    models::{HealthStatus, Issue, Theme},
    modules::{Generator, GeneratorType, Strategy, strategy::PipelineStep, traits::*},
};
use std::{fs, path::PathBuf};

/// Config generator for fzf utility
pub struct FzfGenerator;

impl Identifiable for FzfGenerator {
    fn name(&self) -> &'static str {
        "fzf"
    }

    fn generator_type(&self) -> GeneratorType {
        GeneratorType::Tool
    }
}

impl PathResolvable for FzfGenerator {
    fn base_file_name(&self) -> String {
        "fzf.sh".into()
    }

    fn file_name(&self, theme: &str) -> String {
        format!("{}.sh", theme.to_lowercase())
    }

    fn link_path(&self, paths: &IrisPaths, _theme: &str) -> PathBuf {
        self.zshrc_path(paths)
    }
}

impl Generator for FzfGenerator {
    fn strategy(&self) -> Strategy {
        Strategy::Pipeline { steps: vec![] }
    }

    fn pipeline_steps(&self, paths: &IrisPaths, theme: &Theme) -> Vec<PipelineStep> {
        let cache_file: PathBuf = self.cache_path(paths, &theme.name);
        let zshrc: PathBuf = self.zshrc_path(paths);
        let ppath: String = crate::utils::pretty_path(&cache_file);

        vec![
            PipelineStep::GenerateFile {
                template_name: self.name().into(),
                destination: cache_file,
            },
            PipelineStep::InjectBlock {
                file_path: zshrc,
                marker: "fzf".into(),
                content: format!("[ -f {0} ] && source {0}", ppath),
            },
            PipelineStep::RunCommand {
                program: "zsh".into(),
                args: vec!["-c".into(), "source ~/.zshrc".into()],
                silent: true,
            },
        ]
    }
}

impl FzfGenerator {
    fn zshrc_path(&self, paths: &IrisPaths) -> PathBuf {
        paths
            .config
            .parent()
            .and_then(|p| p.parent())
            .unwrap_or(&paths.config)
            .join(".zshrc")
    }

    /// Universal helper for removing `fzf` marker from every config path (e.g., ~/.zshrc)
    fn remove_fzf_marker(&self, target: &PathBuf) -> anyhow::Result<()> {
        if !target.exists() {
            return Ok(());
        }

        let content = fs::read_to_string(target)?;
        let cleaned = crate::utils::remove_marker(&content, "fzf");

        if content != cleaned {
            let backup = target.with_extension("zshrc.bak");
            let rollback_guard = crate::guards::FsRollbackGuard::new(target.clone(), backup);
            fs::write(target, cleaned.trim())?;
            rollback_guard.commit();
        }

        Ok(())
    }
}

impl Diagnosable for FzfGenerator {
    fn health_check(&self, paths: &IrisPaths, theme: &str) -> HealthStatus {
        if !self.is_installed() {
            return HealthStatus::error(Issue::BinaryNotFound);
        }

        let zshrc: PathBuf = self.zshrc_path(paths);
        if !zshrc.exists() {
            return HealthStatus::error(Issue::ConfigMissing);
        }

        let content: String = match fs::read_to_string(&zshrc) {
            Ok(c) => c,
            Err(_) => return HealthStatus::error(Issue::ConfigMissing),
        };
        let start_marker: String = format!("# [iris:begin:{}]", self.name());
        let end_marker: String = format!("# [iris:end:{}]", self.name());

        if !content.contains(&start_marker) || !content.contains(&end_marker) {
            return HealthStatus::warn(Issue::MarkerMissing);
        }

        if !content.contains("fzf") {
            return HealthStatus::warn(Issue::ImportMissing);
        }

        if !theme.is_empty() {
            let cache_file: PathBuf = self.cache_path(paths, theme);
            if !cache_file.exists() {
                return HealthStatus::warn(Issue::CacheMissing);
            }
        }

        HealthStatus::Ok
    }
}

impl Cleanable for FzfGenerator {
    fn cleanup(&self, paths: &IrisPaths) -> anyhow::Result<()> {
        let zshrc: PathBuf = self.zshrc_path(paths);
        self.remove_fzf_marker(&zshrc)?;

        let fzf_dir: PathBuf = paths.generators.join(self.name());
        if fzf_dir.exists() {
            fs::remove_dir_all(fzf_dir)?;
        }

        Ok(())
    }

    fn remove_theme(&self, paths: &IrisPaths, theme_name: &str) -> anyhow::Result<()> {
        let theme_name_lower: String = theme_name.to_lowercase();
        let cache_file: PathBuf = self.cache_path(paths, &theme_name_lower);

        if cache_file.exists() {
            fs::remove_file(&cache_file)?;
        }

        Ok(())
    }
}

impl Diffable for FzfGenerator {
    fn ideal_content(&self, paths: &IrisPaths, theme: &str) -> anyhow::Result<String> {
        if theme.is_empty() {
            return Ok(String::new());
        }

        let cache_file: PathBuf = self.cache_path(paths, theme);
        let ppath: String = crate::utils::pretty_path(&cache_file);
        Ok(format!("[ -f {} ] && source {}", ppath, ppath))
    }
}

/// Tests for fzf generator
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::IrisContext;

    /// Unit-tests for fzf
    mod unit {
        use super::*;

        #[test]
        fn should_return_metadata_for_fzf() {
            let generator = FzfGenerator;
            assert_eq!(generator.name(), "fzf");
            assert_eq!(generator.generator_type(), GeneratorType::Tool);
            assert_eq!(generator.file_name("vesper"), "vesper.sh");
        }

        #[test]
        fn should_handle_path_resolution_for_fzf() {
            let (_temp_dir, ctx) = IrisContext::mock();
            let generator = FzfGenerator;
            let theme = "tokyonight";

            let expected_zshrc = ctx
                .paths
                .config
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join(".zshrc");
            assert_eq!(generator.zshrc_path(&ctx.paths), expected_zshrc);
            assert_eq!(generator.link_path(&ctx.paths, theme), expected_zshrc);

            let expected_cache_path = ctx.paths.generators.join("fzf/tokyonight.sh");
            assert_eq!(generator.cache_path(&ctx.paths, theme), expected_cache_path);

            assert_eq!(generator.template_path(), "tools/fzf");
        }

        #[test]
        fn should_build_valid_render_context_for_fzf() {
            let generator = FzfGenerator;
            let (_, ctx) = IrisContext::mock();
            let theme: Theme = Theme::mock();
            let ctx = ctx.engine(&theme).build_context(&generator).unwrap();

            assert!(ctx.get("ansi").unwrap().is_array());
        }

        #[test]
        fn should_apply_theme_for_fzf() {
            let (_, ctx) = IrisContext::with_templates(vec![(
                "tools/fzf",
                "export FZF_DEFAULT_OPTS=\"--color='bg:-1,fg:{{ fg }}'",
            )]);
            let generator = FzfGenerator;
            let theme: Theme = Theme::mock();

            let mut activity = ctx.log.step("Test", false).muted();
            ctx.engine(&theme)
                .execute_apply(&generator, &mut activity)
                .unwrap();

            let cache_file = generator.cache_path(&ctx.paths, &theme.name);
            assert!(cache_file.exists(), "Cache file was not created");

            let content = fs::read_to_string(cache_file).unwrap();
            assert!(content.contains("export FZF_DEFAULT_OPTS="));
        }

        #[test]
        fn should_clear_generated_files_for_fzf() {
            let (_, ctx) = IrisContext::mock();
            let generator = FzfGenerator;
            let theme: Theme = Theme::mock();

            let mut activity = ctx.log.step("Test", false).muted();
            ctx.engine(&theme)
                .execute_apply(&generator, &mut activity)
                .unwrap();

            let fzf_script = generator.cache_path(&ctx.paths, &theme.name);
            assert!(fzf_script.exists(), "Cache file should exist after apply");

            generator.cleanup(&ctx.paths).unwrap();
            assert!(!fzf_script.exists());
        }

        #[test]
        fn should_remove_theme_for_fzf() {
            let (_, ctx) = IrisContext::mock();
            let generator = FzfGenerator;
            let mut theme: Theme = Theme::mock();
            theme.name = "test-theme".into();

            let mut activity = ctx.log.step("Test", false).muted();
            ctx.engine(&theme)
                .execute_apply(&generator, &mut activity)
                .unwrap();

            let fzf_script = generator.cache_path(&ctx.paths, &theme.name);
            assert!(fzf_script.exists());

            generator.remove_theme(&ctx.paths, &theme.name).unwrap();
            assert!(
                !fzf_script.exists(),
                "remove_theme should delete the fzf theme script"
            );
        }
    }

    /// Integration tests for fzf
    mod integration {
        use super::*;
        use crate::skip_if_not_installed;

        #[test]
        fn should_return_health_ok_for_fzf() {
            skip_if_not_installed!(FzfGenerator);

            let (_, ctx) = IrisContext::mock();
            let generator = FzfGenerator;
            let theme: Theme = Theme::mock();

            let mut activity = ctx.log.step("Test", true).muted();
            ctx.engine(&theme)
                .execute_apply(&generator, &mut activity)
                .unwrap();

            let status = generator.health_check(&ctx.paths, &theme.name);
            assert!(status.is_ok(), "Expected Ok, got: {status}");
        }

        #[test]
        fn should_return_health_error_no_zshrc_for_fzf() {
            skip_if_not_installed!(FzfGenerator);

            let (_, ctx) = IrisContext::mock();
            let generator = FzfGenerator;
            let status = generator.health_check(&ctx.paths, &ctx.state.theme.current_theme);

            assert!(status.is_error(), "Expected Error, got: {status}");
        }

        #[test]
        fn should_return_health_error_not_sourced_for_fzf() {
            skip_if_not_installed!(FzfGenerator);

            let (tmp_dir, ctx) = IrisContext::mock();
            let generator = FzfGenerator;
            let root = tmp_dir.path();
            let zshrc_path = root.join(".zshrc");

            fs::write(&zshrc_path, "alias ls='ls --color=auto'").unwrap();

            let status = generator.health_check(&ctx.paths, &ctx.state.theme.current_theme);
            assert!(status.is_warning(), "Expected Warning, got: {status}");
        }

        #[test]
        fn should_return_health_error_cache_file_missing_for_fzf() {
            skip_if_not_installed!(FzfGenerator);

            let (_, ctx) = IrisContext::mock();
            let generator = FzfGenerator;
            let theme: Theme = Theme::mock();

            let zshrc = generator.link_path(&ctx.paths, "");
            let cache_file = generator.cache_path(&ctx.paths, &theme.name);

            fs::create_dir_all(zshrc.parent().unwrap()).unwrap();
            fs::write(
                &zshrc,
                format!(
                    "# [iris:begin:fzf]\nsource \"{}\"\n# [iris:end:fzf]",
                    cache_file.display()
                ),
            )
            .unwrap();

            if cache_file.exists() {
                fs::remove_file(&cache_file).unwrap();
            }

            let status = generator.health_check(&ctx.paths, &theme.name);
            assert!(status.is_warning(), "Expected Warning, got: {status}");
        }

        #[test]
        fn should_fix_source_line_injection_for_fzf() {
            skip_if_not_installed!(FzfGenerator);

            let (_, ctx) = IrisContext::with_templates(vec![(
                "tools/fzf",
                "export FZF_DEFAULT_OPTS=\"--color='bg:-1,fg:{{ fg }}'",
            )]);
            let generator = FzfGenerator;
            let theme: Theme = Theme::mock();

            let zshrc = generator.link_path(&ctx.paths, "");
            fs::create_dir_all(zshrc.parent().unwrap()).unwrap();
            fs::write(&zshrc, "# Initial zshrc\n").unwrap();

            let mut activity = ctx.log.step("Test", false).muted();
            let engine = ctx.engine(&theme);

            let cache_file = generator.cache_path(&ctx.paths, &theme.name);
            if let Some(parent) = cache_file.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&cache_file, "echo 'test'").unwrap();

            let status = generator.health_check(&ctx.paths, &theme.name);
            assert!(status.is_warning(), "Expected warning, got: {status}");

            engine
                .execute_fix(&generator, &status, &mut activity)
                .unwrap();

            let updated_content = fs::read_to_string(&zshrc).unwrap();
            assert!(updated_content.contains("[iris:begin:fzf]"));

            let final_status = generator.health_check(&ctx.paths, &theme.name);
            assert!(final_status.is_ok(), "Should be Ok, got: {final_status}");
        }

        #[test]
        fn should_fix_missing_cache_for_fzf() {
            skip_if_not_installed!(FzfGenerator);

            let (_, ctx) = IrisContext::mock();
            let generator = FzfGenerator;
            let theme: Theme = Theme::mock();

            let zshrc = generator.link_path(&ctx.paths, "");
            let cache_file = generator.cache_path(&ctx.paths, &theme.name);

            fs::create_dir_all(zshrc.parent().unwrap()).unwrap();
            fs::write(
                &zshrc,
                format!(
                    "[iris:begin:fzf]\nsource \"{}\"\n[iris:end:fzf]",
                    cache_file.display()
                ),
            )
            .unwrap();

            let status = HealthStatus::error(Issue::CacheMissing);
            let mut activity = ctx.log.step("Test", false).muted();
            let engine = ctx.engine(&theme);
            engine
                .execute_fix(&generator, &status, &mut activity)
                .unwrap();

            assert!(cache_file.exists(), "Cache file should be recreated");
        }
    }
}
