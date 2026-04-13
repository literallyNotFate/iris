use crate::{
    core::IrisContext,
    models::Palette,
    modules::{Generator, GeneratorType},
    utils::{self},
};
use anyhow::{Context, Result};
use colored::Colorize;
use std::{fs, path::PathBuf};

/// Config generator for Alacritty terminal
pub struct AlacrittyGenerator;

impl Generator for AlacrittyGenerator {
    fn name(&self) -> &str {
        "alacritty"
    }

    fn generator_type(&self) -> GeneratorType {
        GeneratorType::Terminal
    }

    fn target_file_name(&self, _theme: &str) -> String {
        "current_theme.toml".into()
    }

    fn apply(&self, p: &Palette, ctx: &IrisContext) -> Result<()> {
        let alacritty_dir: PathBuf = self.resolve_config_directory();
        let theme_file_name: String = self.target_file_name("");
        let cache_file: PathBuf = ctx.paths.cache.join("alacritty").join(&theme_file_name);
        let link_path: PathBuf = alacritty_dir.join(&theme_file_name);

        let render_ctx = self.build_render_context(p);
        let content: String = ctx.templater.render(&self.template_path(), &render_ctx)?;

        if let Some(parent) = cache_file.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create cache directory for {}", self.name()))?;
        }
        if !alacritty_dir.exists() {
            ctx.log.info(&format!(
                "Creating {} config directory...",
                "alacritty".bold()
            ));
            fs::create_dir_all(&alacritty_dir)?;
        }

        fs::write(&cache_file, content)
            .with_context(|| format!("Failed to write alacritty cache to {:?}", cache_file))?;

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
        c.insert("white", &p.white);
        c.insert("sel", &p.sel);
        c.insert("ansi", &p.ansi);
        c
    }

    fn setup_hint(&self) -> Option<String> {
        let alacritty_dir: PathBuf = self.resolve_config_directory();
        let main_config: PathBuf = alacritty_dir.join("alacritty.toml");
        let theme_path: PathBuf = alacritty_dir.join(self.target_file_name(""));
        let import_line: String = format!("import = [\"{}\"]", theme_path.display());

        if !main_config.exists() {
            return Some(format!(
                "No config found. Create {} and add:\n      {}",
                main_config.display().to_string().cyan(),
                import_line.yellow()
            ));
        }

        let content = fs::read_to_string(&main_config).unwrap_or_default();
        if !content.contains("current_theme.toml") {
            return Some(format!(
                "Add this line to your {}:\n      {}",
                main_config.display().to_string().cyan(),
                import_line.yellow()
            ));
        }

        None
    }
}

/// Unit-tests for alacritty
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::create_test_context;
    use tempdir::TempDir;

    #[test]
    fn should_return_alacritty_metadata() {
        let generator = AlacrittyGenerator;
        assert_eq!(generator.name(), "alacritty");
        assert_eq!(generator.generator_type(), GeneratorType::Terminal);
        assert_eq!(generator.target_file_name("nord"), "current_theme.toml");
    }

    #[test]
    fn should_build_valid_render_context() {
        let generator = AlacrittyGenerator;
        let p = Palette::mock();
        let ctx = generator.build_render_context(&p);

        assert_eq!(
            ctx.get("bg")
                .expect("bg not found in context")
                .as_str()
                .unwrap(),
            p.bg
        );
        assert_eq!(
            ctx.get("fg")
                .expect("fg not found in context")
                .as_str()
                .unwrap(),
            p.fg
        );
        assert!(ctx.contains_key("ansi"));
    }

    #[test]
    fn should_generate_setup_hint_for_alacritty() {
        let generator = AlacrittyGenerator;
        let temp_dir: TempDir = TempDir::new("alacritty_test").unwrap();

        temp_env::with_vars(
            vec![
                ("XDG_CONFIG_HOME", Some(temp_dir.path())),
                ("HOME", Some(temp_dir.path())),
            ],
            || {
                let alacritty_dir = generator.resolve_config_directory();
                let main_config = alacritty_dir.join("alacritty.toml");

                let hint_no_config = generator.setup_hint();
                assert!(hint_no_config.is_some());
                assert!(hint_no_config.unwrap().contains("No config found"));

                fs::create_dir_all(&alacritty_dir).unwrap();
                fs::write(&main_config, "[window]\nopacity = 0.9").unwrap();
                let hint_no_import = generator.setup_hint();
                assert!(hint_no_import.is_some());
                assert!(hint_no_import.unwrap().contains("import = ["));

                fs::write(&main_config, "import = [\"current_theme.toml\"]").unwrap();
                assert!(generator.setup_hint().is_none());
            },
        );
    }

    #[test]
    fn should_apply_theme_for_alacritty() {
        if which::which("alacritty").is_err() {
            return;
        }

        let (tmp_dir, ctx) = create_test_context();
        let generator = AlacrittyGenerator;
        let p = Palette::mock();

        temp_env::with_vars(
            vec![
                ("XDG_CONFIG_HOME", Some(tmp_dir.path())),
                ("HOME", Some(tmp_dir.path())),
            ],
            || {
                let result = generator.apply(&p, &ctx);
                assert!(result.is_ok(), "Apply failed: {:?}", result.err());

                let cache_file = ctx.paths.cache.join("alacritty").join("current_theme.toml");
                assert!(cache_file.exists(), "Theme missing in Iris cache");

                let alacritty_dir = generator.resolve_config_directory();
                let link_path = alacritty_dir.join("current_theme.toml");
                assert!(
                    link_path.exists(),
                    "Symlink missing in Alacritty config dir"
                );
                assert!(cache_file.exists());

                let content = fs::read_to_string(cache_file).unwrap();
                assert!(content.contains(&format!("background = \"{}\"", p.bg)));
                assert!(content.contains(&format!("black   = \"{}\"", p.ansi[0])));
                assert!(content.contains(&format!("white   = \"{}\"", p.ansi[15])));
            },
        );
    }
}
