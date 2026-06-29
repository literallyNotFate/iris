use crate::{
    commands::apply_theme,
    core::{IrisContext, ThemeOrchestrator},
    models::Theme,
    utils,
};

/// Handle theme toggle command
pub fn exec(ctx: &mut IrisContext) -> anyhow::Result<()> {
    use colored::Colorize;

    let (target_name, is_fallback) = match &ctx.state.theme.previous_theme {
        Some(prev) => (prev.clone(), false),
        None => (ctx.state.theme.fallback_theme.clone(), true),
    };

    let old_theme: String = ctx.state.theme.current_theme.clone();
    if is_fallback {
        println!(
            "\n{}  No theme history found, using fallback: {}",
            "󰁯".blue(),
            utils::capitalize(&target_name).green().bold()
        );
    } else {
        println!(
            "\n{}  {} {} {} {}",
            "󰑐".bright_blue().bold(),
            "Toggling theme:".bold(),
            utils::capitalize(&old_theme).dimmed(),
            "󰁔".yellow().bold(),
            utils::capitalize(&target_name).magenta().bold()
        );
    }

    println!();

    let orchestrator = ThemeOrchestrator::new(&ctx.paths, &ctx.log);
    let theme_obj: Theme = orchestrator.load_theme(&target_name, false, true, &ctx.state)?;

    apply_theme(&theme_obj, ctx)
}
