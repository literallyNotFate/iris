use crate::{
    core::IrisContext,
    models::Palette,
    modules::{Generator, GeneratorType},
    utils::{self},
};
use anyhow::{Context, Result};
use colored::Colorize;
use std::{fs, path::PathBuf};

/// Config generator for btop utility
pub struct BtopGenerator;

impl Generator for BtopGenerator {
    fn name(&self) -> &str {
        "btop"
    }

    fn generator_type(&self) -> GeneratorType {
        GeneratorType::System
    }

    fn target_file_name(&self, theme: &str) -> String {
        format!("{}.theme", theme)
    }

    fn resolve_config_directory(&self) -> PathBuf {
        dirs::home_dir()
            .map(|p| p.join(".config").join("btop").join("themes"))
            .unwrap_or_else(|| PathBuf::from(".config/btop/themes"))
    }

    fn apply(&self, p: &Palette, ctx: &IrisContext) -> Result<()> {
        let display_name: String = utils::capitalize(&p.name);
        let themes_dir: PathBuf = self.resolve_config_directory();
        let theme_file_name: String = self.target_file_name(&p.name);

        let cache_file = ctx.paths.cache.join("btop_themes").join(&theme_file_name);
        let btop_theme_link: PathBuf = themes_dir.join(format!("{}.theme", &p.name));

        let render_ctx = self.build_render_context(p);
        let content: String = ctx.templater.render(&self.template_path(), &render_ctx)?;

        if let Some(parent) = cache_file.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create cache directory for {}", self.name()))?;
        }

        ctx.log.info(&format!(
            "Theme {} generated in cache.",
            display_name.yellow()
        ));

        fs::write(&cache_file, content)
            .with_context(|| format!("Failed to write btop theme to {:?}", cache_file))?;

        if !themes_dir.exists() {
            ctx.log.info(&format!(
                "Creating {} config directory...",
                self.name().bold()
            ));
            fs::create_dir_all(&themes_dir)?;
        }

        if btop_theme_link.exists() || btop_theme_link.is_symlink() {
            fs::remove_file(&btop_theme_link)?;
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            ctx.log.info(&format!(
                "Creating link in btop/themes/ for {}",
                display_name.yellow()
            ));
            symlink(&cache_file, &btop_theme_link).with_context(|| {
                format!(
                    "Failed to create symlink {:?} -> {:?}",
                    btop_theme_link, cache_file
                )
            })?;
        }

        let btop_root = themes_dir.parent().unwrap_or(&themes_dir);
        let conf_path: PathBuf = btop_root.join("btop.conf");

        if conf_path.exists() {
            ctx.log.info(&format!(
                "Setting color_theme = \"{}\" in btop.conf",
                p.name.bold()
            ));
            self.update_btop_conf(&conf_path, &p.name)?;
        } else {
            ctx.log
                .warn("btop.conf not found. Theme linked but not activated.", 3);
        }

        Ok(())
    }

    fn build_render_context(&self, p: &Palette) -> tera::Context {
        let mut c = tera::Context::new();
        c.insert("theme_name", &utils::capitalize(&p.name));
        c.insert("bg", &p.bg);
        c.insert("fg", &p.fg);
        c.insert("sel", &p.sel);
        c.insert("white", &p.white);
        c.insert("comment", &p.comment);
        c.insert("line_hl", &p.line_hl);
        c.insert("keyword", &p.keyword);
        c.insert("type_name", &p.type_name);
        c.insert("func", &p.func);
        c.insert("tag", &p.tag);
        c.insert("string", &p.string);
        c.insert("constant", &p.constant);
        c.insert("attribute", &p.attribute);

        c.insert("green", &p.ansi[2]);
        c.insert("yellow", &p.ansi[3]);
        c.insert("orange", &p.ansi[9]);

        c
    }

    fn setup_hint(&self) -> Option<String> {
        let themes_dir: PathBuf = self.resolve_config_directory();
        let btop_conf: PathBuf = themes_dir.parent()?.join("btop.conf");

        if !btop_conf.exists() {
            return Some(format!(
                "No {} found. Run btop once to generate it, or create it manually at {}.",
                "btop.conf".cyan(),
                btop_conf.display().to_string().dimmed()
            ));
        }

        let content: String = fs::read_to_string(&btop_conf).unwrap_or_default();
        if !content.contains("color_theme") {
            return Some(format!(
                "Iris generated the theme, but you need to enable it in {}:\n      {}",
                "btop.conf".cyan(),
                "color_theme = \"<theme_name>\"".yellow()
            ));
        }

        None
    }
}

impl BtopGenerator {
    /// Update color_theme setting in btop.conf
    fn update_btop_conf(&self, path: &PathBuf, name: &str) -> Result<()> {
        if !path.exists() {
            return Ok(());
        }

        let content = fs::read_to_string(path)?;
        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        let mut updated = false;

        let theme_line = format!("color_theme = \"{}\"", name);

        for line in lines.iter_mut() {
            if line.trim_start().starts_with("color_theme =") {
                *line = theme_line.clone();
                updated = true;
                break;
            }
        }

        if !updated {
            lines.push(theme_line);
        }

        fs::write(path, lines.join("\n"))?;
        Ok(())
    }
}

/// Unit-tests for btop generator
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::create_test_context;
    use tempdir::TempDir;

    #[test]
    fn should_return_btop_metadata() {
        let generator = BtopGenerator;
        assert_eq!(generator.name(), "btop");
        assert_eq!(generator.generator_type(), GeneratorType::System);
        assert_eq!(generator.target_file_name("iris-dark"), "iris-dark.theme");
    }

    #[test]
    fn should_build_valid_render_context() {
        let generator = BtopGenerator;
        let p = Palette::mock();
        let ctx = generator.build_render_context(&p);

        assert_eq!(ctx.get("bg").expect("bg missing").as_str().unwrap(), p.bg);
        assert_eq!(ctx.get("fg").expect("fg missing").as_str().unwrap(), p.fg);
        assert_eq!(
            ctx.get("keyword")
                .expect("keyword missing")
                .as_str()
                .unwrap(),
            p.keyword
        );

        assert!(ctx.contains_key("green"));
        assert!(ctx.contains_key("yellow"));
        assert!(ctx.contains_key("orange"));
        assert!(ctx.contains_key("type_name"));
        assert!(ctx.contains_key("theme_name"));
    }

    #[test]
    fn should_generate_setup_hint_for_btop() {
        let generator = BtopGenerator;
        let temp_dir: TempDir = TempDir::new("btop_test").unwrap();

        temp_env::with_vars(
            vec![
                ("XDG_CONFIG_HOME", Some(temp_dir.path())),
                ("HOME", Some(temp_dir.path())),
            ],
            || {
                let btop_dir = generator
                    .resolve_config_directory()
                    .parent()
                    .unwrap()
                    .to_path_buf();
                let btop_conf = btop_dir.join("btop.conf");

                let hint_no_conf = generator.setup_hint();
                assert!(hint_no_conf.unwrap().contains("btop.conf"));

                fs::create_dir_all(&btop_dir).unwrap();
                fs::write(&btop_conf, "some_setting = true").unwrap();
                let hint_no_key = generator.setup_hint();
                assert!(hint_no_key.unwrap().contains("color_theme ="));

                fs::write(&btop_conf, "color_theme = \"default\"").unwrap();
                assert!(generator.setup_hint().is_none());
            },
        );
    }

    #[test]
    fn should_update_existing_line_or_append() {
        let generator = BtopGenerator;
        let temp_dir: TempDir = TempDir::new("btop_test").unwrap();
        let conf_path = temp_dir.path().join("btop.conf");

        fs::write(
            &conf_path,
            "theme_background = True\ncolor_theme = \"default\"\n",
        )
        .unwrap();
        generator.update_btop_conf(&conf_path, "new-theme").unwrap();
        let content = fs::read_to_string(&conf_path).unwrap();
        assert!(content.contains("color_theme = \"new-theme\""));
        assert!(!content.contains("color_theme = \"default\""));

        fs::write(&conf_path, "theme_background = True\n").unwrap();
        generator
            .update_btop_conf(&conf_path, "only-theme")
            .unwrap();
        let content = fs::read_to_string(&conf_path).unwrap();
        assert!(content.contains("color_theme = \"only-theme\""));
    }

    #[test]
    fn should_apply_theme_and_update_conf() {
        let (tmp_dir, ctx) = create_test_context();
        let generator = BtopGenerator;
        let p = Palette::mock();

        temp_env::with_vars(
            vec![
                ("XDG_CONFIG_HOME", Some(tmp_dir.path())),
                ("HOME", Some(tmp_dir.path())),
            ],
            || {
                let btop_dir = generator
                    .resolve_config_directory()
                    .parent()
                    .unwrap()
                    .to_path_buf();
                let btop_conf = btop_dir.join("btop.conf");

                fs::create_dir_all(&btop_dir).unwrap();
                fs::write(
                    &btop_conf,
                    "graph_symbol = \"braille\"\ncolor_theme = \"old-theme\"\n",
                )
                .unwrap();

                let result = generator.apply(&p, &ctx);
                assert!(result.is_ok());

                let cache_file = ctx.paths.cache.join("btop_themes").join("test-theme.theme");
                assert!(cache_file.exists());

                let updated_content = fs::read_to_string(&btop_conf).unwrap();
                assert!(updated_content.contains(&format!("color_theme = \"{}\"", p.name)));
                assert!(updated_content.contains("graph_symbol = \"braille\""));
            },
        );
    }
}
