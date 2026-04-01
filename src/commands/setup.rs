use crate::core::{IrisContext, IrisSetup};
use colored::*;

/// Handle application setup command
pub fn exec(ctx: &IrisContext) -> anyhow::Result<()> {
    IrisSetup::run(ctx)?;

    println!(
        "\n{}",
        " Iris is now fully configured and ready to go!"
            .green()
            .bold()
    );
    Ok(())
}
