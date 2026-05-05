use crate::{cli::switch::SwitchArgs, commands::apply_theme, core::IrisContext, models::Palette};

/// Handle application switch command
pub fn exec(args: SwitchArgs, ctx: &mut IrisContext) -> anyhow::Result<()> {
    use colored::Colorize;
    let (target_name, is_fb) = ctx.resolve_theme(Some(args.name), args.fallback)?;

    if is_fb {
        println!(
            "\n{}  Using fallback: {}",
            "󰁯".blue(),
            target_name.green().bold()
        );
    }

    println!();
    let palette: Palette = Palette::fetch(
        &target_name,
        args.force,
        true,
        &ctx.paths,
        &ctx.state,
        &ctx.log,
    )?;

    apply_theme(&palette, ctx)
}
