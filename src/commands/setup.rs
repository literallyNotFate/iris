use crate::core::{IrisContext, IrisSetup};

/// Handle application setup command
pub fn exec(ctx: &mut IrisContext) -> anyhow::Result<()> {
    IrisSetup::run(ctx)?;

    ctx.log
        .success("Iris is now fully configured and ready to go!\n");
    Ok(())
}
