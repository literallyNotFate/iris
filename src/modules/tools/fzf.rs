use crate::{
    core::IrisContext,
    models::Palette,
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

    fn is_installed(&self) -> bool {
        let home: PathBuf = dirs::home_dir().unwrap_or_default();
        which::which("fzf").is_ok() || home.join(".zshrc").exists()
    }

    fn apply(&self, p: &Palette, ctx: &IrisContext) -> Result<()> {
        let cache_file: PathBuf = ctx.paths.cache.join(self.target_file_name(&p.name));
        let render_ctx = self.build_render_context(p);
        let content: String = ctx.templater.render(&self.template_path(), &render_ctx)?;

        ctx.log.info(&format!(
            "Generating FZF script in: {}",
            cache_file.display()
        ));

        fs::write(&cache_file, content)
            .with_context(|| format!("Failed to write FZF config to {:?}", cache_file))?;

        ctx.log.info("Colors exported to shell script.");
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

    fn setup_hint(&self) -> Option<String> {
        let cache_file: PathBuf = dirs::home_dir()?
            .join(".cache/iris")
            .join(self.target_file_name(""));
        let zshrc: PathBuf = dirs::home_dir()?.join(".zshrc");

        let source_line: String = format!("source \"{}\"", cache_file.display());

        if zshrc.exists() {
            let content: String = fs::read_to_string(&zshrc).unwrap_or_default();
            if content.contains("fzf.sh") {
                return None;
            }
        }

        Some(format!(
            "fzf theme won't load until you add to {}:\n     {}",
            ".zshrc".cyan(),
            source_line.yellow()
        ))
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
    fn should_generate_setup_hint_for_fzf() {
        let generator = FzfGenerator;
        let temp_dir: TempDir = TempDir::new("fzf_test").unwrap();
        let zshrc_path = temp_dir.path().join(".zshrc");

        temp_env::with_var("HOME", Some(temp_dir.path()), || {
            let hint_no_zshrc = generator.setup_hint();
            assert!(hint_no_zshrc.is_some());

            fs::write(&zshrc_path, "# some config").unwrap();
            let hint_no_source = generator.setup_hint();
            assert!(hint_no_source.unwrap().contains("fzf theme won't load"));

            fs::write(&zshrc_path, "source \"/some/path/fzf.sh\"").unwrap();
            let hint_with_source = generator.setup_hint();
            assert!(hint_with_source.is_none());
        });
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
