use crate::{context::AppContext, models::Palette};
use anyhow::{Context, Result};
#[cfg(unix)]
use std::os::unix::fs::symlink;

/// Apply generated colors config to ghostty
pub fn apply(palette: &Palette, ctx: &AppContext) -> Result<()> {
    let home = dirs::home_dir().context("Home dir not found")?;
    let ghostty_dir = home.join(".config/ghostty");
    let cache_file = ctx.cache_path.join("ghostty.conf");
    let link_path = ghostty_dir.join("current_theme.conf");

    let config_content: String =
        crate::generators::ghostty::build_ghostty_config(palette, &ctx.state.current_theme);

    std::fs::create_dir_all(&ctx.cache_path).context("Failed to create cache directory")?;
    std::fs::write(&cache_file, config_content)
        .with_context(|| format!("Failed to write ghostty cache to {:?}", cache_file))?;

    if !ghostty_dir.exists() {
        std::fs::create_dir_all(&ghostty_dir).ok();
    }

    if link_path.exists() || link_path.is_symlink() {
        std::fs::remove_file(&link_path).ok();
    }

    #[cfg(unix)]
    symlink(&cache_file, &link_path).with_context(|| {
        format!(
            "Failed to create symlink {:?} -> {:?}",
            link_path, cache_file
        )
    })?;

    Ok(())
}
