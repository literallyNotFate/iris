use crate::{context::AppContext, models::Theme};
use anyhow::{Context, Result};
use std::{fs, os::unix::fs::symlink, path::PathBuf};

/// Generate ghostty config based on selected theme
pub fn apply(theme: &Theme, ctx: &AppContext) -> Result<()> {
    let home = dirs::home_dir().context("Home dir not found")?;
    let ghostty_dir: PathBuf = home.join(".config/ghostty");
    let cache_file: PathBuf = ctx.cache_path.join("ghostty.conf");
    let link_path: PathBuf = ghostty_dir.join("current_theme.conf");

    let mut cfg = String::new();
    let fix = |v: &String| {
        if v.starts_with('#') {
            v.clone()
        } else {
            format!("#{}", v)
        }
    };

    for (k, v) in &theme.colors {
        let key = if k == "cursor" { "cursor-color" } else { k };
        cfg.push_str(&format!("{} = {}\n", key, fix(v)));
    }

    let mut palette: Vec<_> = theme.palette.iter().collect();
    palette.sort_by_key(|(k, _)| k.parse::<u32>().unwrap_or(0));
    for (idx, val) in palette {
        cfg.push_str(&format!("palette = {}={}\n", idx, fix(val)));
    }

    fs::create_dir_all(cache_file.parent().unwrap())?;
    fs::write(&cache_file, cfg)
        .with_context(|| format!("Failed to write ghostty cache to {:?}", cache_file))?;

    if link_path.exists() || link_path.is_symlink() {
        fs::remove_file(&link_path).ok();
    }

    if !ghostty_dir.exists() {
        fs::create_dir_all(&ghostty_dir).ok();
    }

    symlink(&cache_file, &link_path).with_context(|| {
        format!(
            "Failed to create symlink {:?} -> {:?}",
            link_path, cache_file
        )
    })?;

    Ok(())
}
