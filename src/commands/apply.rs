use crate::{
    commands::HealthStatus,
    core::IrisContext,
    models::{NvimStrategy, Palette},
    utils,
};
use colored::Colorize;

/// Handle application apply command
pub fn exec(
    generator: String,
    theme: Option<String>,
    fallback: bool,
    ctx: &mut IrisContext,
) -> anyhow::Result<()> {
    let mut theme_to_apply: String = theme
        .clone()
        .map(|s| s.to_lowercase())
        .unwrap_or_else(|| ctx.state.current_theme.clone());

    let mut is_fallback_applied: bool = false;
    if theme_to_apply.is_empty() {
        if fallback && !ctx.state.fallback_theme.is_empty() {
            theme_to_apply = ctx.state.fallback_theme.clone();
            is_fallback_applied = true;
        } else {
            anyhow::bail!(
                "No theme specified and no global theme active. Use `iris switch <name>` or `--fallback`."
            );
        }
    }

    let exists: bool = Palette::exists(&theme_to_apply, ctx);
    let is_default_strat: bool = matches!(ctx.state.nvim, NvimStrategy::Default);

    if !exists
        || (is_default_strat
            && !NvimStrategy::get_builtin_themes().contains(&theme_to_apply)
            && !ctx
                .paths
                .palettes
                .join(format!("{}.json", theme_to_apply))
                .exists())
    {
        if fallback {
            theme_to_apply = ctx.state.fallback_theme.clone();
            is_fallback_applied = true;

            if !Palette::exists(&theme_to_apply, ctx) {
                anyhow::bail!(
                    "Both requested theme and fallback `{}` are unavailable.",
                    theme_to_apply
                );
            }
        } else {
            anyhow::bail!(
                "Theme `{}` is unavailable (not found or restricted by strategy). Use `--fallback`.",
                theme_to_apply.yellow().bold()
            );
        }
    }

    println!(
        "\n {}  {} {}",
        "󰷉".yellow().bold(),
        "Standalone apply:".bold(),
        theme_to_apply.magenta()
    );

    if is_fallback_applied {
        println!(
            "    {} Using fallback: {} because `{}` was unavailable",
            "󰁯".blue(),
            theme_to_apply.green().bold(),
            theme.unwrap_or_else(|| "current".to_string()).dimmed()
        );
    }

    let palette: Palette = Palette::fetch(&theme_to_apply, false, ctx)?;
    let g = ctx
        .registry
        .get(&generator)
        .ok_or_else(|| anyhow::anyhow!("Unknown generator: `{}`", generator.red()))?;

    let status = g.health_check(ctx);
    if !matches!(status, HealthStatus::Ok) {
        let mut t_fix = ctx.log.step(
            &format!("{} Repairing {}...", "󰒓".green(), g.name().cyan()),
            1,
        );

        g.fix(&status, &palette, ctx)?;
        t_fix.done(true);
    }

    let mut t_apply = ctx.log.step(
        &format!(
            "{} Applying {} to {}...",
            "󱓞".magenta().bold(),
            utils::capitalize(&theme_to_apply).yellow().bold(),
            g.name().cyan().bold()
        ),
        1,
    );

    g.apply(&palette, ctx)?;
    t_apply.done(true);

    println!(
        "\n {}  Successfully updated {}.",
        "󰄬".green(),
        g.name().bold().cyan()
    );

    Ok(())
}
