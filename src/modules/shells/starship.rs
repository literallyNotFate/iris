use crate::{
    core::IrisContext,
    models::Palette,
    modules::ConfigGenerator,
    utils::{self, Status},
};
use anyhow::{Context, Result};
use colored::Colorize;
use std::fs;

/// Config generator for starship
pub struct StarshipGenerator;

impl ConfigGenerator for StarshipGenerator {
    fn name(&self) -> &str {
        "starship"
    }

    fn is_installed(&self) -> bool {
        let home = dirs::home_dir().unwrap_or_default();
        which::which("starship").is_ok() || home.join(".config/starship").exists()
    }

    fn apply(&self, p: &Palette, ctx: &IrisContext) -> Result<()> {
        let theme_name = &ctx.state.current_theme;
        let task = Status::step(&format!("Configuring {}...", self.name().cyan()), 2);

        let starship_config = dirs::home_dir()
            .context("Cannot get the home directory!")?
            .join(".config/starship/starship.toml");

        let palette_block = self.build_config(p, theme_name);

        let existing = if starship_config.exists() {
            fs::read_to_string(&starship_config)
                .with_context(|| format!("Failed to read {:?}", starship_config))?
        } else {
            format!("palette = \"{}\"\n", theme_name)
        };

        let palette_header = format!("[palettes.{}]", theme_name);
        let updated = if existing.contains(&palette_header) {
            replace_palette_block(&existing, &palette_header, &palette_block)
        } else {
            let with_palette_key = set_palette_key(&existing, theme_name);
            format!("{}\n{}", with_palette_key.trim_end(), palette_block)
        };

        task.info(&format!(
            "Palette {} written to starship config.",
            utils::capitalize(theme_name).yellow()
        ));

        fs::write(&starship_config, updated)
            .with_context(|| format!("Failed to write {:?}", starship_config))?;

        task.done(Some(&format!("{} sync complete.", self.name().cyan())));
        Ok(())
    }

    fn setup_hint(&self) -> Option<String> {
        let config = dirs::home_dir()?.join(".config/starship/starship.toml");

        if !config.exists() {
            return Some(format!(
                "No {} found. Create it and add:\n     {}",
                "~/.config/starship/starship.toml".cyan(),
                "palette = \"<theme_name>\"".yellow()
            ));
        }

        let content = fs::read_to_string(&config).unwrap_or_default();
        if !content.contains("palette =") {
            return Some(format!(
                "Theme won't load until you add to {}:\n     {}",
                "starship.toml".cyan(),
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
