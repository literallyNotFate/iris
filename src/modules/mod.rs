pub mod bat;
pub mod fzf;
pub mod ghostty;

use crate::{context::AppContext, models::Palette};
use anyhow::Result;

/// Apply themes to available programs (enabled generators)
pub fn apply_all(palette: &Palette, ctx: &AppContext) -> Result<()> {
    for generator in &ctx.state.enabled_generators {
        match generator.as_str() {
            "ghostty" => ghostty::apply(palette, ctx)?,
            "bat" => bat::apply(palette, ctx)?,
            "fzf" => fzf::apply(palette, ctx)?,
            _ => {}
        }
    }

    Ok(())
}
