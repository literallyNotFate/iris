use crate::{commands::HealthStatus, core::IrisContext, models::Palette, utils};
use colored::Colorize;

/// Handle application health command
pub fn exec(generator: String, theme: Option<String>, ctx: &IrisContext) -> anyhow::Result<()> {
    let theme_to_apply: &str = theme.as_deref().unwrap_or_else(|| {
        if ctx.state.current_theme.is_empty() {
            ""
        } else {
            &ctx.state.current_theme
        }
    });

    if theme_to_apply.is_empty() {
        anyhow::bail!(
            "No theme specified and no global theme active. Use `iris switch <name>` first."
        );
    }

    let is_different: bool = Some(theme_to_apply) != Some(&ctx.state.current_theme);
    println!(
        "\n {}  {} {}",
        "󰷉".yellow().bold(),
        "Standalone apply:".bold(),
        theme_to_apply.magenta()
    );
    println!();

    if is_different && theme.is_some() {
        println!(
            "   Temporary override: using {} instead of system {}\n",
            theme_to_apply.yellow(),
            ctx.state.current_theme.dimmed()
        );
    }

    let palette: Palette = if is_different {
        let mut t = ctx.log.step(
            &format!(
                "{} Fetching palette {}...",
                "󰟶".cyan().bold(),
                theme_to_apply.yellow().bold()
            ),
            1,
        );
        let p = Palette::fetch(theme_to_apply, &ctx)?;
        t.done(true);
        p
    } else {
        Palette::fetch(theme_to_apply, &ctx.silent())?
    };

    let g = ctx.registry.get(&generator).ok_or_else(|| {
        anyhow::anyhow!(
            "Unknown generator: `{}`. Run `iris gen list` for available tools.",
            generator.red()
        )
    })?;

    if !ctx.registry.is_installed(g.name()) {
        ctx.log.warn(
            &format!(
                "Generator `{}` binary not found in PATH. Operation might fail.",
                g.name().bold().cyan()
            ),
            0,
        );
    }

    let mut t = ctx.log.step(
        &format!(
            "{} Checking health for {}...",
            "󰚚 ".green().bold(),
            g.name().cyan()
        ),
        1,
    );

    let status = g.health_check(ctx);
    if matches!(status, HealthStatus::Ok) {
        t.done(true);
    } else {
        t.done(false);

        let mut t_fix = ctx.log.step(
            &format!(
                "{} Repairing {} configuration...",
                "󰒓".green().bold(),
                g.name().cyan()
            ),
            2,
        );

        g.fix(&status, &palette, ctx)?;
        t_fix.done(true);
    }

    let mut t_apply = ctx.log.step(
        &format!(
            "{} Applying {} to {}...",
            "󱓞".magenta().bold(),
            utils::capitalize(theme_to_apply).yellow().bold(),
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
