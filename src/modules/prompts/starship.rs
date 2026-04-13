use crate::{
    core::IrisContext,
    models::Palette,
    modules::{Generator, GeneratorType},
    utils::{self},
};
use anyhow::{Context, Result};
use colored::Colorize;
use std::{env, fs, path::PathBuf};

/// Config generator for starship
pub struct StarshipGenerator;

impl Generator for StarshipGenerator {
    fn name(&self) -> &str {
        "starship"
    }

    fn generator_type(&self) -> GeneratorType {
        GeneratorType::Prompt
    }

    fn target_file_name(&self, _theme: &str) -> String {
        "starship.toml".into()
    }

    fn env_config_directory(&self) -> Option<PathBuf> {
        env::var("STARSHIP_CONFIG").ok().map(PathBuf::from)
    }

    fn is_installed(&self) -> bool {
        which::which("starship").is_ok() || self.resolve_config_directory().exists()
    }

    fn apply(&self, p: &Palette, ctx: &IrisContext) -> Result<()> {
        let config_path: PathBuf = env::var("STARSHIP_CONFIG")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                self.resolve_config_directory()
                    .join(self.target_file_name(&p.name))
            });

        let render_ctx = self.build_render_context(p);
        let palette_block: String = ctx.templater.render(&self.template_path(), &render_ctx)?;
        let palette_header: String = format!("[palettes.{}]", &p.name);

        let existing = if config_path.exists() {
            fs::read_to_string(&config_path)
                .with_context(|| format!("Failed to read {:?}", config_path))?
        } else {
            ctx.log.info(&format!(
                "Creating {} config directory...",
                "starship".bold()
            ));
            if let Some(parent) = config_path.parent() {
                fs::create_dir_all(parent)?;
            }

            format!("palette = \"{}\"\n", &p.name)
        };

        let mut updated: String = set_palette_key(&existing, &p.name);
        updated = replace_palette_block(&updated, &palette_header, &palette_block);

        fs::write(&config_path, updated)
            .with_context(|| format!("Failed to write {:?}", config_path))?;

        ctx.log.info(&format!(
            "Palette {} injected into starship config.",
            utils::capitalize(&p.name).yellow()
        ));

        Ok(())
    }

    fn build_render_context(&self, p: &Palette) -> tera::Context {
        let mut c = tera::Context::new();
        c.insert("theme_name", &p.name);
        c.insert("bg", &p.bg);
        c.insert("fg", &p.fg);
        c.insert("sel", &p.sel);
        c.insert("line_hl", &p.line_hl);
        c.insert("gutter_fg", &p.gutter_fg);
        c.insert("ansi", &p.ansi);
        c
    }

    fn setup_hint(&self) -> Option<String> {
        let config: PathBuf = env::var("STARSHIP_CONFIG")
            .map(PathBuf::from)
            .unwrap_or_else(|_| self.resolve_config_directory().join("starship.toml"));

        if !config.exists() {
            return Some(format!(
                "Starship config not found. Create {} and add:\n      {}",
                config.display().to_string().cyan(),
                "palette = \"<theme_name>\"".yellow()
            ));
        }

        let content = fs::read_to_string(&config).unwrap_or_default();
        if !content.contains("palette =") {
            return Some(format!(
                "Theme won't load until you add to {}:\n      {}",
                config.display().to_string().cyan(),
                "palette = \"<theme_name>\"".yellow()
            ));
        }

        None
    }
}

/// Replaces [palettes.<name>] block with new content
fn replace_palette_block(content: &str, header: &str, new_block: &str) -> String {
    let start = match content.find(header) {
        Some(i) => i,
        None => return format!("{}\n{}", content.trim_end(), new_block),
    };

    let after = &content[start + header.len()..];
    let end = after
        .find("\n[")
        .map(|i| start + header.len() + i)
        .unwrap_or(content.len());

    format!(
        "{}{}{}",
        &content[..start],
        new_block.trim(),
        &content[end..]
    )
}

/// Updates or inserts `palette = "<name>"` line
fn set_palette_key(content: &str, name: &str) -> String {
    let new_line = format!("palette = \"{}\"", name);
    if content.contains("palette =") {
        content
            .lines()
            .map(|l| {
                if l.trim_start().starts_with("palette =") {
                    new_line.clone()
                } else {
                    l.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        format!("{}\n{}", new_line, content)
    }
}

/// Unit-tests for starship generator
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::create_test_context;
    use tempdir::TempDir;

    #[test]
    fn should_return_starship_metadata() {
        let generator = StarshipGenerator;
        assert_eq!(generator.name(), "starship");
        assert_eq!(generator.generator_type(), GeneratorType::Prompt);
        assert_eq!(generator.target_file_name("any"), "starship.toml");
    }

    #[test]
    fn should_build_valid_render_context() {
        let generator = StarshipGenerator;
        let p = Palette::mock();
        let ctx = generator.build_render_context(&p);

        assert_eq!(ctx.get("bg").expect("bg missing").as_str().unwrap(), p.bg);
        assert_eq!(ctx.get("fg").expect("fg missing").as_str().unwrap(), p.fg);

        let ansi = ctx
            .get("ansi")
            .expect("ansi array missing")
            .as_array()
            .expect("ansi should be an array");
        assert!(ansi.len() >= 16);
        assert!(ctx.contains_key("gutter_fg"));
        assert!(ctx.contains_key("sel"));
        assert!(ctx.contains_key("line_hl"));
        assert!(ctx.contains_key("theme_name"));
    }

    #[test]
    fn should_handle_palette_key_set_for_starship() {
        let content = "line1 = true\nline2 = false";
        let updated = set_palette_key(content, "my-theme");
        assert!(updated.starts_with("palette = \"my-theme\""));

        let content_with_key = "palette = \"old\"\nother = 1";
        let updated_key = set_palette_key(content_with_key, "new");
        assert!(updated_key.contains("palette = \"new\""));
        assert!(!updated_key.contains("palette = \"old\""));
    }

    #[test]
    fn should_handle_replace_palette_block_for_starship() {
        let header = "[palettes.test]";
        let new_block = "[palettes.test]\ncolor = \"red\"\n";
        let content = "[directory]\ntruncation_length = 3";
        let result = replace_palette_block(content, header, new_block);
        assert!(result.contains(new_block));

        let complex_content = "[palettes.test]\nold = true\n\n[character]\nsymbol = \">\"";
        let result_complex = replace_palette_block(complex_content, header, new_block);
        assert!(result_complex.contains(new_block));
        assert!(result_complex.contains("[character]"));
        assert!(!result_complex.contains("old = true"));
    }

    #[test]
    fn should_generate_setup_hint_for_starship() {
        let generator = StarshipGenerator;
        let temp_dir: TempDir = TempDir::new("starship_test").unwrap();
        let config_path = temp_dir.path().join("starship.toml");

        temp_env::with_var("STARSHIP_CONFIG", Some(&config_path), || {
            let hint = generator.setup_hint();
            assert!(hint.unwrap().contains("Starship config not found"));

            fs::write(&config_path, "[character]\nsuccess_symbol = \">\"").unwrap();
            let hint_no_key = generator.setup_hint();
            assert!(hint_no_key.unwrap().contains("palette ="));

            fs::write(&config_path, "palette = \"some-theme\"").unwrap();
            assert!(generator.setup_hint().is_none());
        });
    }

    #[test]
    fn should_apply_theme_for_starship() {
        if which::which("starship").is_err() {
            return;
        }

        let (tmp_dir, ctx) = create_test_context();
        let generator = StarshipGenerator;
        let p = Palette::mock();
        let config_path = tmp_dir.path().join("starship.toml");

        temp_env::with_var("STARSHIP_CONFIG", Some(&config_path), || {
            fs::write(&config_path, "[directory]\nstyle = \"blue\"\n").unwrap();

            let result = generator.apply(&p, &ctx);
            assert!(result.is_ok());

            let final_content = fs::read_to_string(&config_path).unwrap();

            assert!(final_content.contains(&format!("palette = \"{}\"", p.name)));
            assert!(final_content.contains(&format!("[palettes.{}]", p.name)));
            assert!(final_content.contains("[directory]"));
        });
    }
}
