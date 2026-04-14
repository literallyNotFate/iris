use crate::{commands::HealthStatus, core::IrisContext, models::Palette, utils};
use colored::Colorize;

/// Handle application health command
pub fn exec(generator: String, ctx: &IrisContext) -> anyhow::Result<()> {
    println!("\n {}  {}", "󰷉".yellow().bold(), "Standalone apply".bold());
    println!();

    let palette: Palette = Palette::fetch(&ctx.state.current_theme, &ctx.log.as_quiet())?;
    let target = ctx.registry.get(&generator);

    match target {
        Some(g) => {
            if !ctx.registry.is_installed(g.name()) {
                ctx.log.warn(
                    &format!(
                        "󱘲  Generator '{}' binary not found in PATH.",
                        g.name().bold().cyan()
                    ),
                    0,
                );
            }

            let mut t = ctx
                .log
                .step(&format!("󰚚  Checking health for {}...", g.name().cyan()), 1);
            let status = g.health_check(ctx);
            if matches!(status, HealthStatus::Ok) {
                t.done(true);
            } else {
                t.done(false);
                g.fix(&status, &palette, ctx)?;
            }

            let mut t = ctx.log.step(
                &format!(
                    "󱓞  Applying {} to {}...",
                    utils::capitalize(&ctx.state.current_theme).yellow().bold(),
                    g.name().cyan()
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
