pub mod bat;
pub mod fzf;
pub mod ghostty;

use crate::{context::AppContext, models::Palette, status::Status};
use anyhow::Result;
use colored::Colorize;

/// Apply themes to available programs (enabled generators)
pub fn apply_all(palette: &Palette, ctx: &AppContext) -> Result<()> {
    println!();
    Status::step(
        &format!(
            "Applying palette to {} targets...",
            ctx.state.enabled_generators.len()
        ),
        0,
    );

    for generator in &ctx.state.enabled_generators {
        let name = generator.as_str();
        Status::step(&format!("Configuring {}...", name.cyan()), 1);

        let result = match name {
            "ghostty" => ghostty::apply(palette, ctx),
            "bat" => bat::apply(palette, ctx),
            "fzf" => fzf::apply(palette, ctx),
            _ => {
                Status::error(&format!("Unknown generator: {}", name), 1);
                continue;
            }
        };

        match result {
            Ok(_) => Status::success(&format!("{} is ready!", name.cyan()), 1),
            Err(e) => {
                Status::error(&format!("Failed to configure {}: {}", name.red(), e), 1);
                return Err(e);
            }
        }
    }

    Ok(())
}
