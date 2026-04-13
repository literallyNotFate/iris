use crate::{
    core::IrisContext,
    models::Palette,
    modules::{Generator, GeneratorType},
    utils::{self},
};
use anyhow::{Context, Result};
use colored::Colorize;
use std::{fs, path::PathBuf};

/// Config generator for ghostty terminal
pub struct GhosttyGenerator;

impl Generator for GhosttyGenerator {
    fn name(&self) -> &str {
        "ghostty"
    }

    fn generator_type(&self) -> GeneratorType {
        GeneratorType::Terminal
    }

    fn target_file_name(&self, _theme: &str) -> String {
        "current_theme.conf".into()
    }

    fn apply(&self, p: &Palette, ctx: &IrisContext) -> Result<()> {
        let ghostty_dir: PathBuf = self.resolve_config_directory();
        let theme_file_name: String = self.target_file_name(&p.name);
        let cache_file: PathBuf = ctx.paths.cache.join("ghostty").join(&theme_file_name);
        let link_path: PathBuf = ghostty_dir.join(&theme_file_name);

        let render_ctx = self.build_render_context(p);
        let content: String = ctx.templater.render(&self.template_path(), &render_ctx)?;

        if let Some(parent) = cache_file.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create cache directory for {}", self.name()))?;
        }
        if !ghostty_dir.exists() {
            ctx.log.info(&format!(
                "Creating {} config directory...",
                "ghostty".bold()
            ));
            fs::create_dir_all(&ghostty_dir)?;
        }

        fs::write(&cache_file, content)
            .with_context(|| format!("Failed to write ghostty cache to {:?}", cache_file))?;

        if link_path.exists() || link_path.is_symlink() {
            fs::remove_file(&link_path)?;
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            ctx.log.info(&format!(
                "Linking cache to {} config...",
                self.name().bold()
            ));
            symlink(&cache_file, &link_path)
                .with_context(|| format!("Failed to link {:?} -> {:?}", link_path, cache_file))?;
        }

        Ok(())
    }

    fn build_render_context(&self, p: &Palette) -> tera::Context {
        let mut c = tera::Context::new();
        c.insert("theme_name", &utils::capitalize(&p.name));
        c.insert("bg", &p.bg);
        c.insert("fg", &p.fg);
        c.insert("sel_bg", &p.sel);
        c.insert("sel_fg", &p.bg);
        c.insert("cursor", &p.white);
        c.insert("ansi", &p.ansi);
        c
    }

    fn setup_hint(&self) -> Option<String> {
        let ghostty_dir: PathBuf = self.resolve_config_directory();
        let config_path: PathBuf = ghostty_dir.join("config");
        let import_line: String = format!("config-file = {}", self.target_file_name(""));

        if !config_path.exists() {
            return Some(format!(
                "No config found. Create {} and add:\n      {}",
                config_path.display().to_string().cyan(),
                import_line.yellow()
            ));
        }

        let content: String = fs::read_to_string(&config_path).unwrap_or_default();
        if !content.contains(&import_line) {
            return Some(format!(
                "Theme won't load until you add to {}:\n      {}",
                config_path.display().to_string().cyan(),
                import_line.yellow()
            ));
        }

        None
    }
}

/// Unit-tests for ghostty
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::create_test_context;
    use tempdir::TempDir;

    #[test]
    fn should_return_ghostty_metadata() {
        let generator = GhosttyGenerator;
        assert_eq!(generator.name(), "ghostty");
        assert_eq!(generator.generator_type(), GeneratorType::Terminal);
        assert_eq!(generator.target_file_name("any"), "current_theme.conf");
    }

    #[test]
    fn should_build_valid_render_context() {
        let generator = GhosttyGenerator;
        let p = Palette::mock();
        let ctx = generator.build_render_context(&p);

        assert_eq!(ctx.get("bg").unwrap().as_str().unwrap(), p.bg);
        assert!(ctx.get("ansi").unwrap().is_array());
    }

    #[test]
    fn should_generate_setup_hint_for_ghostty() {
        let generator = GhosttyGenerator;
        let temp_dir: TempDir = TempDir::new("ghostty_test").unwrap();

        temp_env::with_vars(
            vec![
                ("XDG_CONFIG_HOME", Some(temp_dir.path())),
                ("HOME", Some(temp_dir.path())),
            ],
            || {
                let ghostty_dir = generator.resolve_config_directory();
                let config_file = ghostty_dir.join("config");

                let hint_no_dir = generator.setup_hint();
                assert!(
                    hint_no_dir.is_some(),
                    "Should show hint if config directory is missing"
                );

                fs::create_dir_all(&ghostty_dir).unwrap();
                fs::write(&config_file, "font-size = 14\n# empty config").unwrap();

                let hint_no_import = generator.setup_hint();
                assert!(
                    hint_no_import.is_some(),
                    "Should show hint if config exists but lacks import"
                );
                assert!(
                    hint_no_import
                        .unwrap()
                        .contains("config-file = current_theme.conf"),
                    "Hint message should contain the required import line"
                );

                fs::write(
                    &config_file,
                    "font-size = 14\nconfig-file = current_theme.conf",
                )
                .unwrap();
                let hint_perfect = generator.setup_hint();
                assert!(
                    hint_perfect.is_none(),
                    "Hint should be None when config is valid"
                );
            },
        );
    }

    #[test]
    fn should_apply_theme_for_ghostty() {
        if which::which("ghostty").is_err() {
            return;
        }

        let (tmp_dir, ctx) = create_test_context();
        let generator = GhosttyGenerator;
        let p = Palette::mock();

        temp_env::with_vars(
            vec![
                ("XDG_CONFIG_HOME", Some(tmp_dir.path())),
                ("HOME", Some(tmp_dir.path())),
            ],
            || {
                let result = generator.apply(&p, &ctx);
                assert!(result.is_ok(), "Failed to apply: {:?}", result.err());

                let cache_file = ctx.paths.cache.join("ghostty").join("current_theme.conf");
                assert!(cache_file.exists());

                let content = fs::read_to_string(cache_file).unwrap();
                assert!(content.contains("background ="));
                assert!(content.contains("palette = 0="));
                assert!(content.contains("palette = 15="));
            },
        );
    }
}
