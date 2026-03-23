use crate::{
    core::{IrisContext, IrisSetup},
    models::Palette,
    modules,
    utils::Status,
};
use anyhow::Result;
use colored::*;

/// Handle application setup command
pub fn exec(ctx: &IrisContext) -> Result<()> {
    IrisSetup::run(ctx)?;

    println!();
    let sync_task = Status::step("Performing initial sync...", 0);

    let theme: String = Palette::current()?;
    let palette: Palette = Palette::fetch(&theme)?;

    modules::apply_all(&palette, ctx)?;

    sync_task.done(Some("Initial sync complete."));
    println!(
        "\n{}",
        " Iris is now fully configured and ready to go!"
            .green()
            .bold()
    );
    Ok(())
}
