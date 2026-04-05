use crate::{
    core::IrisContext,
    models::Palette,
    modules::Generator,
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
        let theme_name = &ctx.state.current_theme;
        let config_path: PathBuf = env::var("STARSHIP_CONFIG")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                self.resolve_config_directory()
                    .join(self.target_file_name(theme_name))
            });

        let palette_block: String = self.build_config(p, theme_name);
        let palette_header: String = format!("[palettes.{}]", theme_name);

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

            format!("palette = \"{}\"\n", theme_name)
        };

        let mut updated = set_palette_key(&existing, theme_name);
        updated = replace_palette_block(&updated, &palette_header, &palette_block);

        fs::write(&config_path, updated)
            .with_context(|| format!("Failed to write {:?}", config_path))?;

        ctx.log.info(&format!(
            "Palette {} written to starship config.",
            utils::capitalize(theme_name).yellow()
        ));

        Ok(())
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

impl StarshipGenerator {
    /// Build starship theme palette
    pub fn build_config(&self, p: &Palette, name: &str) -> String {
        let a = &p.ansi;
        format!(
            r#"
[palettes.{name}]
base     = "{bg}"
mantle   = "{mantle}"
text     = "{fg}"
subtext0 = "{subtext0}"
surface  = "{surface}"
overlay  = "{overlay}"
red      = "{red}"
green    = "{green}"
yellow   = "{yellow}"
blue     = "{blue}"
mauve    = "{mauve}"
teal     = "{teal}"
peach    = "{peach}"
foam     = "{foam}"
gold     = "{gold}"
"#,
            name = name,
            bg = p.bg,
            mantle = a[0],
            fg = p.fg,
            subtext0 = p.gutter_fg,
            surface = p.sel,
            overlay = p.line_hl,
            red = a[1],
            green = a[2],
            yellow = a[3],
            blue = a[4],
            mauve = a[5],
            teal = a[6],
            peach = a[9],
            foam = a[14],
            gold = a[11],
        )
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

    format!("{}{}{}", &content[..start], new_block, &content[end..])
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
