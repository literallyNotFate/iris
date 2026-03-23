pub mod bat;
pub mod fzf;
pub mod ghostty;

pub use bat::BatGenerator;
pub use fzf::FzfGenerator;
pub use ghostty::GhosttyGenerator;

use crate::{core::IrisContext, models::Palette, utils::Status};
use anyhow::Result;
use colored::Colorize;

/// Main trait for all generators
pub trait ConfigGenerator {
    /// Returns name of the generator (e.g "ghostty")
    fn name(&self) -> &str;

    /// Checks whether this tool is installed
    fn is_installed(&self) -> bool {
        which::which(self.name()).is_ok()
    }

    /// Logic of applying the theme (file writing, building cache etc)
    fn apply(&self, p: &Palette, ctx: &IrisContext) -> Result<()>;
}

/// Apply themes to available programs (enabled generators)
pub fn apply_all(palette: &Palette, ctx: &IrisContext) -> Result<()> {
    println!();
    let total_task = Status::step(
        &format!("Applying palette to {} targets...", ctx.generators.len()),
        0,
    );

    for generator in &ctx.generators {
        let app_task = Status::step(&format!("Configuring {}...", generator.name().cyan()), 1);
        match generator.apply(palette, ctx) {
            Ok(_) => app_task.done(Some(&format!("{} is ready!", generator.name().cyan()))),
            Err(e) => {
                app_task.fail(&format!("Failed to configure {}: {}", generator.name(), e));

                return Err(e);
            }
        }
    }

    total_task.done(Some("All targets updated successfully."));
    Ok(())
}
