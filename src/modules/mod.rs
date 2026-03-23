pub mod bat;
pub mod fzf;
pub mod ghostty;

use crate::{context::AppContext, models::Palette, status::Status};
use anyhow::Result;
use colored::Colorize;

/// Apply themes to available programs (enabled generators)
pub fn apply_all(palette: &Palette, ctx: &AppContext) -> Result<()> {
    println!();
    let total_task = Status::step(
        &format!(
            "Applying palette to {} targets...",
            ctx.state.enabled_generators.len()
        ),
        0,
    );

    for generator in &ctx.state.enabled_generators {
        let name = generator.as_str();
        let app_task = Status::step(&format!("Configuring {}...", name.cyan()), 1);

        let result = match name {
            "ghostty" => ghostty::apply(palette, ctx),
            "bat" => bat::apply(palette, ctx),
            "fzf" => fzf::apply(palette, ctx),
            _ => {
                app_task.fail(&format!("Unknown generator: {}", name));
                continue;
            }
        };

        match result {
            Ok(_) => app_task.done(Some(&format!("{} is ready!", name.cyan()))),
            Err(e) => {
                app_task.fail(&format!("{}", e));
                return Err(e);
            }
        }
    }

    total_task.done(Some("All targets updated successfully."));
    Ok(())
}
