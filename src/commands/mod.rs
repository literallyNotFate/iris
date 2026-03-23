use crate::{cli::Commands, core::IrisContext};

pub mod setup;
pub mod status;
pub mod switch;

/// Handles all commands
pub fn handle(command: Commands, ctx: &mut IrisContext) -> anyhow::Result<()> {
    match command {
        Commands::Init => setup::exec(ctx)?,
        Commands::Switch { name } => switch::exec(name, ctx)?,
        Commands::Status => status::exec(ctx)?,
    }

    Ok(())
}
