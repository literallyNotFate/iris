use crate::{
    core::IrisContext,
    models::Palette,
    modules::{Generator, GeneratorType},
    utils,
};
use anyhow::{Context, Result};
use colored::Colorize;
use std::{fs, path::PathBuf};

/// Config generator for yazi
pub struct YaziGenerator;

impl Generator for YaziGenerator {
    fn name(&self) -> &str {
        "yazi"
    }

    fn generator_type(&self) -> GeneratorType {
        GeneratorType::Tool
    }

    fn target_file_name(&self, _theme: &str) -> String {
        "theme.toml".into()
    }

    fn apply(&self, p: &Palette, ctx: &IrisContext) -> Result<()> {
        let theme_name: &String = &p.name;

        let render_ctx: tera::Context = self.build_render_context(p);
        let content: String = ctx.templater.render(&self.template_path(), &render_ctx)?;

        let yazi_dir: PathBuf = self.resolve_config_directory();
        if !yazi_dir.exists() {
            ctx.log.info(&format!(
                "Creating {} config directory...",
                self.name().bold()
            ));
            fs::create_dir_all(&yazi_dir)?;
        }

        let cache_file: PathBuf = ctx
            .paths
            .cache
            .join("yazi_themes")
            .join(format!("{}.toml", theme_name));
        let theme_link: PathBuf = yazi_dir.join(self.target_file_name(theme_name));

        fs::create_dir_all(cache_file.parent().unwrap())?;
        fs::write(&cache_file, content)?;

        ctx.log.info(&format!(
            "Theme {} generated in cache.",
            utils::capitalize(theme_name).yellow()
        ));

        if theme_link.exists() || theme_link.is_symlink() {
            fs::remove_file(&theme_link)?;
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            ctx.log.info(&format!(
                "Linking theme.toml to {} config...",
                self.name().bold()
            ));
            symlink(&cache_file, &theme_link).with_context(|| {
                format!(
                    "Failed to create symlink {:?} -> {:?}",
                    theme_link, cache_file
                )
            })?;
        }

        Ok(())
    }

    fn build_render_context(&self, p: &Palette) -> tera::Context {
        let mut c = tera::Context::new();

        c.insert("theme_name", &utils::capitalize(&p.name));
        c.insert("bg", &p.bg);
        c.insert("fg", &p.fg);
        c.insert("white", &p.white);
        c.insert("comment", &p.comment);
        c.insert("gutter_fg", &p.gutter_fg);
        c.insert("ansi", &p.ansi);
        c.insert("sel", &p.sel);

        let line_hl = if p.line_hl == "#cccccc" {
            &p.sel
        } else {
            &p.line_hl
        };
        c.insert("line_hl", line_hl);

        c.insert("red", &p.ansi[1]);
        c.insert("green", &p.ansi[2]);
        c.insert("orange", &p.ansi[3]);
        c.insert("blue", &p.ansi[4]);
        c.insert("magenta", &p.ansi[5]);
        c.insert("teal", &p.ansi[6]);
        c.insert("tan", &p.ansi[7]);
        c.insert("br_red", &p.ansi[9]);
        c.insert("br_green", &p.ansi[10]);
        c.insert("br_orange", &p.ansi[11]);
        c.insert("br_blue", &p.ansi[12]);
        c.insert("br_magenta", &p.ansi[13]);
        c.insert("br_teal", &p.ansi[14]);

        c
    }

    fn setup_hint(&self) -> Option<String> {
        let yazi_dir: PathBuf = self.resolve_config_directory();

        if !yazi_dir.exists() {
            return Some(format!(
                "No {} found — make sure Yazi is installed and run it once to initialize config.",
                yazi_dir.display().to_string().cyan(),
            ));
        }

        None
    }
}

/// Unit-tests for yazi generator
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::create_test_context;
    use temp_env;
    use tempdir::TempDir;

    #[test]
    fn should_return_yazi_metadata() {
        let generator = YaziGenerator;
        assert_eq!(generator.name(), "yazi");
        assert_eq!(generator.generator_type(), GeneratorType::Tool);
        assert_eq!(generator.target_file_name("any"), "theme.toml");
    }

    #[test]
    fn should_build_valid_render_context() {
        let generator = YaziGenerator;
        let mut p = Palette::mock();

        p.line_hl = "#123456".to_string();
        let ctx = generator.build_render_context(&p);
        assert_eq!(ctx.get("line_hl").unwrap().as_str().unwrap(), "#123456");

        p.line_hl = "#cccccc".to_string();
        p.sel = "#ff0000".to_string();
        let ctx = generator.build_render_context(&p);

        assert_eq!(ctx.get("line_hl").unwrap().as_str().unwrap(), "#ff0000");
        assert!(ctx.get("red").is_some());
        assert!(ctx.get("br_teal").is_some());
    }

    #[test]
    fn should_generate_setup_hint_for_yazi() {
        let generator = YaziGenerator;
        let temp_dir: TempDir = TempDir::new("yazi_hint").unwrap();
        let fake_config_root = temp_dir.path().join("config");
        fs::create_dir_all(&fake_config_root).unwrap();

        temp_env::with_vars(
            vec![
                ("XDG_CONFIG_HOME", Some(&fake_config_root)),
                ("HOME", Some(&temp_dir.into_path())),
            ],
            || {
                let resolved = generator.resolve_config_directory();

                let hint = generator.setup_hint();
                assert!(
                    hint.is_some(),
                    "Hint should be Some for path: {:?}",
                    resolved
                );

                fs::create_dir_all(&resolved).unwrap();
                assert!(generator.setup_hint().is_none());
            },
        );
    }

    #[test]
    fn should_apply_theme_for_yazi() {
        if which::which("yazi").is_err() {
            return;
        }

        let (tmp_dir, ctx) = create_test_context();
        let generator = YaziGenerator;
        let p = Palette::mock();

        temp_env::with_vars(
            vec![
                ("XDG_CONFIG_HOME", Some(tmp_dir.path())),
                ("HOME", Some(tmp_dir.path())),
            ],
            || {
                let result = generator.apply(&p, &ctx);
                assert!(result.is_ok(), "Apply failed: {:?}", result.err());

                let expected_yazi_dir = generator.resolve_config_directory();
                let yazi_theme_link = expected_yazi_dir.join("theme.toml");

                assert!(
                    yazi_theme_link.exists(),
                    "Symlink missing at {:?}. Check if resolve_config_directory is consistent!",
                    yazi_theme_link
                );

                let cache_content = fs::read_to_string(yazi_theme_link).unwrap();
                assert!(cache_content.contains("generated by Iris"));
            },
        );
    }
}
