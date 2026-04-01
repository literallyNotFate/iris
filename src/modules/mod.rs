pub mod alacritty;
pub mod bat;
pub mod btop;
pub mod fzf;
pub mod ghostty;
pub mod starship;
pub mod yazi;

pub use alacritty::AlacrittyGenerator;
pub use bat::BatGenerator;
pub use btop::BtopGenerator;
pub use fzf::FzfGenerator;
pub use ghostty::GhosttyGenerator;
pub use starship::StarshipGenerator;
pub use yazi::YaziGenerator;

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

    /// Optional post-apply hint (e.g. "add import to config")
    fn setup_hint(&self) -> Option<String> {
        None
    }
}

/// Get all generators
pub fn all_generators() -> Vec<Box<dyn ConfigGenerator>> {
    vec![
        Box::new(crate::modules::GhosttyGenerator),
        Box::new(crate::modules::BatGenerator),
        Box::new(crate::modules::FzfGenerator),
        Box::new(crate::modules::BtopGenerator),
        Box::new(crate::modules::YaziGenerator),
        Box::new(crate::modules::AlacrittyGenerator),
        Box::new(crate::modules::StarshipGenerator),
    ]
}

/// Return generator based on string name
pub fn generator(name: &str) -> Option<Box<dyn ConfigGenerator>> {
    all_generators().into_iter().find(|g| g.name() == name)
}

/// Apply themes to available programs (enabled generators)
pub fn apply_all(palette: &Palette, ctx: &IrisContext) -> Result<()> {
    println!();
    let generators_len = ctx.generators.len().to_string();
    let total_task = Status::step(
        &format!("Applying palette to {} targets...", generators_len.green()),
        0,
    );

    for generator in &ctx.generators {
        if let Err(e) = generator.apply(palette, ctx) {
            total_task.fail(&format!(
                "Failed at {}: {}",
                generator.name().cyan(),
                e.to_string().red()
            ));
            return Err(e);
        }

        if let Some(hint) = generator.setup_hint() {
            Status::warn(&hint, 3);
        }
    }

    total_task.done(Some("All targets updated successfully."));
    Ok(())
}
