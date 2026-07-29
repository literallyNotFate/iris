use crate::{cli::switch::SwitchArgs, core::IrisContext, models::Theme, service::ThemeService};

/// Handle application switch (main entry point for changing themes)
pub fn exec(args: SwitchArgs, ctx: &mut IrisContext) -> anyhow::Result<()> {
    use colored::Colorize;
    let (target_name, is_fb) = ctx.resolve_theme(Some(args.name), args.fallback)?;

    if is_fb {
        println!(
            "\n{}  Using fallback: {}",
            "󰁯".blue(),
            crate::utils::capitalize(&target_name).green().bold()
        );
    }

    println!();

    let service: ThemeService = ThemeService::new(&ctx.paths, &ctx.log);
    let theme_obj: Theme = service.load_theme(&target_name, args.force, true, &ctx.state)?;

    crate::commands::apply_theme(&theme_obj, ctx)
}
