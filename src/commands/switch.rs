use crate::{cli::switch::SwitchArgs, commands::apply_theme, core::IrisContext, models::Palette};
use colored::Colorize;

/// Handle application switch command
pub fn exec(args: SwitchArgs, ctx: &mut IrisContext) -> anyhow::Result<()> {
    let (target_name, is_fb) = ctx.resolve_theme(Some(args.name), args.fallback)?;

    if is_fb {
        println!(
            "\n {}  Using fallback: {}",
            "󰁯".blue(),
            target_name.green().bold()
        );
    }

    if args.force && !Palette::exists(&target_name, &ctx.paths, &ctx.state) {
        println!("\n {}  Deep fetch via Neovim...", "󰚔 ".cyan());
    }

    apply_theme(&target_name, args.force, ctx)
}
