pub mod fzf;
pub mod ghostty;

use crate::{context::AppContext, models::Theme};
use anyhow::Result;

/// Apply themes to available programs (enabled generators)
pub fn apply_all(theme: &Theme, ctx: &AppContext) -> Result<()> {
    for app in &ctx.state.enabled_generators {
        match app.as_str() {
            "fzf" => fzf::apply(theme, ctx)?,
            "ghostty" => ghostty::apply(theme, ctx)?,
            _ => println!(" Unknown generator: {}", app),
        }
    }

    Ok(())
}
