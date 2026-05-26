use crate::{
    cli::switch::SwitchArgs,
    commands::apply_theme,
    core::{IrisContext, ThemeOrchestrator},
    models::Theme,
    utils,
};

/// Handle application switch (main entry point for changing themes)
pub fn exec(args: SwitchArgs, ctx: &mut IrisContext) -> anyhow::Result<()> {
    use colored::Colorize;
    let (target_name, is_fb) = ctx.resolve_theme(Some(args.name), args.fallback)?;

    if is_fb {
        println!(
            "\n{}  Using fallback: {}",
            "󰁯".blue(),
            utils::capitalize(&target_name).green().bold()
        );
    }

    println!();

    let orchestrator: ThemeOrchestrator = ThemeOrchestrator::new(&ctx.paths, &ctx.log);
    let theme_obj: Theme = orchestrator.load_theme(&target_name, args.force, true, &ctx.state)?;

    apply_theme(&theme_obj, ctx)
}
