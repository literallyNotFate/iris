use crate::{
    commands::HealthStatus,
    core::IrisContext,
    models::Palette,
    modules::{Generator, GeneratorType},
    utils::{self},
};
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

    fn cache_path(&self, ctx: &IrisContext, _theme_name: &str) -> PathBuf {
        ctx.paths.bin.join(self.target_file_name(""))
    }

    fn link_path(&self, ctx: &IrisContext, _theme_name: &str) -> PathBuf {
        ctx.paths
            .config
            .parent()
            .and_then(|p| p.parent())
            .unwrap_or(&ctx.paths.config)
            .join(".zshrc")
    }

    fn apply(&self, p: &Palette, ctx: &IrisContext) -> anyhow::Result<()> {
        ctx.log.info(&format!(
            "Generating {} script in: {}",
            self.name().bold().cyan(),
            utils::pretty_path(&self.cache_path(ctx, "")).magenta()
        ));
        self.ensure_cache_file(p, ctx)?;

        ctx.log.info(&format!(
            "{} theme applied to {}",
            utils::capitalize(&p.name).yellow(),
            self.name().bold().cyan()
        ));
        Ok(())
    }

    fn build_render_context(&self, p: &Palette) -> tera::Context {
        let mut c = tera::Context::new();
        let strip = |hex: &str| hex.trim_start_matches('#').to_string();

        c.insert("theme_name", &utils::capitalize(&p.name));
        c.insert("fg", &strip(&p.fg));
        c.insert("bg", &strip(&p.bg));
        c.insert("accent", &strip(&p.ansi[3]));
        c.insert("match_c", &strip(&p.ansi[5]));
        c.insert("dimmed", &strip(&p.ansi[8]));

        c
    }

    fn health_check(&self, ctx: &IrisContext) -> HealthStatus {
        if !self.is_installed() {
            return HealthStatus::Warning("fzf binary not found".into());
        }

        let zshrc: PathBuf = self.link_path(ctx, "");
        let cache_file: PathBuf = self.cache_path(ctx, "");

        if !zshrc.exists() {
            return HealthStatus::Error {
                message: ".zshrc not found".into(),
                fix_hint: Some("fzf theme requires a shell config to source the colors".into()),
            };
        }

        let content: String = fs::read_to_string(&zshrc).unwrap_or_default();
        if !content.contains("fzf.sh") {
            return HealthStatus::Error {
                message: "fzf.sh is not sourced in .zshrc".into(),
                fix_hint: Some(format!(
                    "Add 'source \"{}\"' to your .zshrc",
                    cache_file.display()
                )),
            };
        }

        if !cache_file.exists() {
            return HealthStatus::Error {
                message: "fzf theme file missing from cache".into(),
                fix_hint: Some("Run `iris sync` to regenerate it".into()),
            };
        }

        HealthStatus::Ok
    }

    fn fix(&self, status: &HealthStatus, p: &Palette, ctx: &IrisContext) -> anyhow::Result<()> {
        match status {
            HealthStatus::Error { message, .. } => {
                if message.contains("missing from cache") {
                    ctx.log
                        .step(
                            &format!("Restoring missing {} theme file...", self.name().bold()),
                            2,
                        )
                        .done(true);
                    return self.apply(p, &ctx.silent());
                }

                if message.contains("not sourced") {
                    ctx.log
                        .step(
                            &format!("Injecting source line into {}...", ".zshrc".magenta()),
                            2,
                        )
                        .done(true);
                    self.inject_source_line(ctx)?;
                }

                self.apply(p, &ctx.silent())
            }

            _ => {
                ctx.log
                    .step(
                        &format!("Re-applying {} configuration...", self.name().bold()),
                        2,
                    )
                    .done(true);
                self.apply(p, &ctx.silent())
            }
        }
    }
}

impl FzfGenerator {
    fn ensure_cache_file(&self, p: &Palette, ctx: &IrisContext) -> anyhow::Result<PathBuf> {
        let cache_file: PathBuf = self.cache_path(ctx, &p.name);
        let render_ctx = self.build_render_context(p);
        let content: String = ctx.templater.render(&self.template_path(), &render_ctx)?;

        fs::create_dir_all(cache_file.parent().unwrap())?;
        fs::write(&cache_file, content)?;
        Ok(cache_file)
    }

    fn inject_source_line(&self, ctx: &IrisContext) -> anyhow::Result<()> {
        let zshrc: PathBuf = self.link_path(ctx, "");
        let cache_file: PathBuf = self.cache_path(ctx, "");
        let source_line: String = format!(
            "[ -f \"{0}\" ] && source \"{0}\" # iris:fzf",
            cache_file.display()
        );

        let content: String = fs::read_to_string(&zshrc).unwrap_or_default();
        if content.contains("# iris:fzf") {
            return Ok(());
        }

        let mut new_content: String = content.trim_end().to_string();
        new_content.push_str("\n\n# Import fzf theme from iris\n");
        new_content.push_str(&source_line);
        new_content.push('\n');

        fs::write(&zshrc, new_content)?;
        Ok(())
    }
}

/// Unit-tests for fzf generator
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::tests::create_test_context;

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
        let p = Palette::mock();
        let render_ctx = generator.build_render_context(&p);
        let fg = render_ctx.get("fg").unwrap().as_str().unwrap();

        assert!(!fg.starts_with('#'));
    }

    #[test]
    fn should_return_health_ok_for_fzf() {
        let (_, ctx) = create_test_context();
        let generator = FzfGenerator;
        let p = Palette::mock();

        generator.apply(&p, &ctx).unwrap();

        let zshrc_path = generator.link_path(&ctx, "");
        let cache_file = generator.cache_path(&ctx, "any");

        fs::create_dir_all(zshrc_path.parent().unwrap()).unwrap();
        fs::write(&zshrc_path, format!("source \"{}\"", cache_file.display())).unwrap();

        let status = generator.health_check(&ctx);
        assert!(
            matches!(status, HealthStatus::Ok),
            "Expected Ok, got {:?}",
            status
        );
    }

    #[test]
    fn should_return_health_error_no_zshrc_for_fzf() {
        let (_, ctx) = create_test_context();
        let generator = FzfGenerator;
        let status = generator.health_check(&ctx);

        match status {
            HealthStatus::Error { message, .. } => {
                assert!(message.contains(".zshrc not found"));
            }
            _ => panic!("Expected Error due to missing .zshrc"),
        }
    }

    #[test]
    fn should_return_health_error_not_sourced_for_fzf() {
        let (tmp_dir, ctx) = create_test_context();
        let generator = FzfGenerator;
        let root = tmp_dir.path();
        let zshrc_path = root.join(".zshrc");

        fs::write(&zshrc_path, "alias ls='ls --color=auto'").unwrap();

        let status = generator.health_check(&ctx);
        match status {
            HealthStatus::Error { message, .. } => {
                assert!(message.contains("not sourced"));
            }
            _ => panic!("Expected Error because fzf.sh is not in .zshrc"),
        }
    }

    #[test]
    fn should_return_health_error_cache_file_missing_for_fzf() {
        let (_tmp_dir, ctx) = create_test_context();
        let generator = FzfGenerator;

        let zshrc = generator.link_path(&ctx, "");
        let cache_file = generator.cache_path(&ctx, "");

        fs::create_dir_all(zshrc.parent().unwrap()).unwrap();
        fs::write(&zshrc, format!("source {:?} # iris:fzf", cache_file)).unwrap();

        if cache_file.exists() {
            fs::remove_file(&cache_file).unwrap();
        }

        let status = generator.health_check(&ctx);

        match status {
            HealthStatus::Error { message, .. } => {
                assert!(message.contains("missing from cache") || message.contains("not found"));
            }
            _ => panic!("Expected HealthStatus::Error, but got {:?}", status),
        }
    }

    #[test]
    fn should_apply_fzf_theme_to_cache() {
        let (_, ctx) = create_test_context();
        let generator = FzfGenerator;
        let p = Palette::mock();

        let result = generator.apply(&p, &ctx);
        assert!(
            result.is_ok(),
            "Apply should be successful, but got: {:?}",
            result.err()
        );

        let cache_file = ctx.paths.bin.join("fzf.sh");
        assert!(cache_file.exists(), "Cache file fzf.sh was not created");

        let content = fs::read_to_string(cache_file).unwrap();
        assert!(content.contains("export FZF_DEFAULT_OPTS="));
        assert!(content.contains(&utils::capitalize(&p.name)));
    }

    #[test]
    fn should_fix_source_line_injection_for_fzf() {
        let (tmp_dir, ctx) = create_test_context();
        let generator = FzfGenerator;
        let p = Palette::mock();
        let root = tmp_dir.path();

        let zshrc = root.join(".zshrc");
        fs::write(&zshrc, "# Initial zshrc\n").unwrap();

        generator.apply(&p, &ctx).unwrap();
        let status = generator.health_check(&ctx);
        assert!(
            matches!(status, HealthStatus::Error { ref message, .. } if message.contains("not sourced")),
            "Expected 'not sourced' error, got {:?}",
            status
        );

        generator
            .fix(&status, &p, &ctx.silent())
            .expect("Fix failed");

        let updated_content = fs::read_to_string(&zshrc).unwrap();
        let cache_file = generator.cache_path(&ctx, &p.name);

        assert!(
            updated_content.contains(&cache_file.to_str().unwrap()),
            "zshrc should now contain the source line for fzf.sh"
        );
        assert!(updated_content.contains("# iris:fzf"));

        let final_status = generator.health_check(&ctx);
        assert!(final_status.is_ok(), "Final status should be Ok");
    }

    #[test]
    fn should_fix_missing_cache_for_fzf() {
        let (_, ctx) = create_test_context();
        let generator = FzfGenerator;
        let p = Palette::mock();

        let zshrc = generator.link_path(&ctx, "");
        let cache_file = generator.cache_path(&ctx, &p.name);

        fs::create_dir_all(zshrc.parent().unwrap()).unwrap();
        fs::write(
            &zshrc,
            format!("source \"{}\" # fzf.sh", cache_file.display()),
        )
        .unwrap();

        let status = HealthStatus::Error {
            message: "missing from cache".into(),
            fix_hint: None,
        };

        generator.fix(&status, &p, &ctx.silent()).unwrap();
        assert!(cache_file.exists(), "Cache file should be recreated");

        let content = fs::read_to_string(cache_file).unwrap();
        assert!(content.contains("export FZF_DEFAULT_OPTS="));
    }
}
