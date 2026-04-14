use crate::{commands::HealthStatus, core::IrisContext, models::Palette, utils};
use colored::Colorize;

/// Handle application health command
pub fn exec(generator: String, theme: Option<String>, ctx: &IrisContext) -> anyhow::Result<()> {
    let theme_to_apply: String = theme
        .clone()
        .or_else(|| {
            if ctx.state.current_theme.is_empty() {
                None
            } else {
                Some(ctx.state.current_theme.clone())
            }
        })
        .ok_or_else(|| anyhow::anyhow!("No theme specified and no global theme active."))?;

    let is_different: bool = Some(&theme_to_apply) != Some(&ctx.state.current_theme);

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
        let p = Palette::fetch(&theme_to_apply, &ctx.log)?;

        t.done(true);
        p
    } else {
        Palette::fetch(&theme_to_apply, &ctx.log.as_quiet())?
    };

    let target = ctx.registry.get(&generator);

    match target {
        Some(g) => {
            if !ctx.registry.is_installed(g.name()) {
                ctx.log.warn(
                    &format!(
                        "Generator '{}' binary not found in PATH.",
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

                let mut t = ctx.log.step(
                    &format!(
                        "{} Repairing {} configuration...",
                        "󰒓".green().bold(),
                        g.name().cyan()
                    ),
                    2,
                );
                g.fix(&status, &palette, ctx)?;
                t.done(true);
            }

            let mut t = ctx.log.step(
                &format!(
                    "{} Applying {} to {}...",
                    "󱓞".magenta().bold(),
                    utils::capitalize(&theme_to_apply).yellow().bold(),
                    g.name().cyan().bold()
                ),
                1,
            );
            g.apply(&palette, ctx)?;
            t.done(true);

            println!(
                "\n {}  Successfully updated {}.",
                "󰄬".green(),
                g.name().bold().cyan()
            );
        }
        None => {
            ctx.log.error(
                &format!(
                    "Unknown generator: '{}'. Run 'iris gen list' for help.",
                    generator.red()
                ),
                0,
            );
        }
    }

    Ok(())
}
