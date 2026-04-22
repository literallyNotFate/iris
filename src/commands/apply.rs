use crate::{
    cli::switch::ApplyArgs,
    core::IrisContext,
    models::{HealthStatus, Palette},
    modules::Generator,
    utils,
};
use colored::Colorize;

/// Handle application apply command
pub fn exec(args: ApplyArgs, ctx: &mut IrisContext) -> anyhow::Result<()> {
    let (theme_name, was_fallback) = ctx.resolve_theme(args.theme.clone(), args.fallback)?;
    render_header(&theme_name, args.theme.as_deref(), was_fallback);

    let palette = Palette::fetch(&theme_name, false, &ctx.paths, &ctx.state, &ctx.log)?;
    let generator = ctx.resolve_generator(&args.generator)?;

    ensure_generator_health(generator, &palette, ctx)?;

    let mut step = ctx.log.step(
        &format!(
            "{} Applying {} to {}...",
            "󱓞".magenta().bold(),
            utils::capitalize(&theme_name).yellow().bold(),
            generator.name().cyan().bold()
        ),
        1,
    );

    generator.apply(&palette, &ctx.paths, &ctx.templater, &ctx.log)?;
    step.done(true);

    println!(
        "\n {}  Successfully updated {}.",
        "󰄬".green(),
        generator.name().bold().cyan()
    );

    Ok(())
}

/// Helper function to ensure generator health
fn ensure_generator_health(
    g: &dyn Generator,
    p: &Palette,
    ctx: &IrisContext,
) -> anyhow::Result<()> {
    let status: HealthStatus = g.health_check(&ctx.paths, &p.name);

    if !status.is_ok() {
        let mut t = ctx.log.step(
            &format!("{} Repairing {}...", "󰒓".green(), g.name().cyan()),
            1,
        );

        g.fix(&status, p, &ctx.paths, &ctx.templater, &ctx.log)?;
        t.done(true);
    }
    Ok(())
}

/// Helper function to render header for apply
fn render_header(theme: &str, original: Option<&str>, is_fallback: bool) {
    println!(
        "\n {}  {} {}",
        "󰷉".yellow().bold(),
        "Standalone apply:".bold(),
        theme.magenta()
    );
    if is_fallback {
        println!(
            "    {} Using fallback: {} because `{}` was unavailable",
            "󰁯".blue(),
            theme.green().bold(),
            original.unwrap_or("current").dimmed()
        );
    }
}
