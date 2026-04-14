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
        ctx.paths.cache.join(self.target_file_name(""))
    }

    fn link_path(&self, _theme_name: &str) -> PathBuf {
        dirs::home_dir().unwrap_or_default().join(".zshrc")
    }

    fn is_installed(&self) -> bool {
        let home: PathBuf = dirs::home_dir().unwrap_or_default();
        which::which("fzf").is_ok() || home.join(".zshrc").exists()
    }

    fn apply(&self, p: &Palette, ctx: &IrisContext) -> anyhow::Result<()> {
        let cache_file: PathBuf = self.cache_path(ctx, &p.name);
        let render_ctx = self.build_render_context(p);
        let content: String = ctx.templater.render(&self.template_path(), &render_ctx)?;

        ctx.log.info(&format!(
            "Generating {} script in: {}",
            self.name().bold(),
            utils::pretty_path(&cache_file).cyan()
        ));

        fs::create_dir_all(cache_file.parent().unwrap())?;
        fs::write(&cache_file, content)?;

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

        let zshrc: PathBuf = self.link_path("");
        let cache_file: PathBuf = self.cache_path(ctx, "");

        if !zshrc.exists() {
            return HealthStatus::Error {
                message: ".zshrc not found".into(),
                fix_hint: Some("fzf theme requires a shell config to source the colors".into()),
            };
        }

        let content: String = fs::read_to_string(&zshrc).unwrap_or_default();
        let source_line: String = format!("fzf.sh");

        if !content.contains(&source_line) {
            return HealthStatus::Error {
                message: "fzf.sh is not sourced in .zshrc".into(),
                fix_hint: Some(format!(
                    "Add 'source \"{}\"' to your .zshrc",
                    cache_file.display()
                )),
            };
        }

        HealthStatus::Ok
    }
}

/// Unit-tests for fzf generator
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::create_test_context;
    use std::fs;
    use tempdir::TempDir;

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
    fn should_check_if_fzf_is_installed() {
        let generator = FzfGenerator;
        let temp_dir: TempDir = TempDir::new("fzf_test").unwrap();
        let zshrc_path = temp_dir.path().join(".zshrc");

        temp_env::with_var("HOME", Some(temp_dir.path()), || {
            fs::write(&zshrc_path, "").unwrap();
            assert!(generator.is_installed());
        });
    }

    #[test]
    fn fzf_health_ok() {
        let (tmp_dir, ctx) = create_test_context();
        let generator = FzfGenerator;
        let root = tmp_dir.path();

        temp_env::with_var("HOME", Some(root), || {
            let zshrc_path = root.join(".zshrc");
            let cache_file = generator.cache_path(&ctx, "any");

            fs::write(&zshrc_path, format!("source \"{}\"", cache_file.display())).unwrap();

            let status = generator.health_check(&ctx);
            assert!(matches!(status, HealthStatus::Ok));
        });
    }

    #[test]
    fn fzf_health_error_no_zshrc() {
        let (tmp_dir, ctx) = create_test_context();
        let generator = FzfGenerator;
        let root = tmp_dir.path();

        temp_env::with_var("HOME", Some(root), || {
            let status = generator.health_check(&ctx);

            match status {
                HealthStatus::Error { message, .. } => {
                    assert!(message.contains(".zshrc not found"));
                }
                _ => panic!("Expected Error due to missing .zshrc"),
            }
        });
    }

    #[test]
    fn fzf_health_error_not_sourced() {
        let (tmp_dir, ctx) = create_test_context();
        let generator = FzfGenerator;
        let root = tmp_dir.path();

        temp_env::with_var("HOME", Some(root), || {
            let zshrc_path = root.join(".zshrc");
            fs::write(&zshrc_path, "alias ls='ls --color=auto'").unwrap();

            let status = generator.health_check(&ctx);
            match status {
                HealthStatus::Error { message, .. } => {
                    assert!(message.contains("not sourced"));
                }
                _ => panic!("Expected Error because fzf.sh is not in .zshrc"),
            }
        });
    }

    #[test]
    fn should_apply_fzf_theme_to_cache() {
        if which::which("fzf").is_err() {
            return;
        }

        let (_tmp_dir, ctx) = create_test_context();
        let generator = FzfGenerator;
        let p = Palette::mock();

        let result = generator.apply(&p, &ctx);
        assert!(
            result.is_ok(),
            "Apply should be successful, but got: {:?}",
            result.err()
        );

        let cache_file = ctx.paths.cache.join("fzf.sh");
        assert!(cache_file.exists(), "Cache file fzf.sh was not created");

        let content = fs::read_to_string(cache_file).unwrap();
        assert!(content.contains("export FZF_DEFAULT_OPTS="));
        assert!(content.contains(&utils::capitalize(&p.name)));
    }
}
