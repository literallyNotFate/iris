use crate::{
    cli::switch::ApplyArgs,
    core::{IrisContext, ThemeOrchestrator},
    models::Theme,
    modules::Generator,
};
use colored::Colorize;

/// Handle application apply command
pub fn exec(args: ApplyArgs, ctx: &mut IrisContext) -> anyhow::Result<()> {
    let (theme_name, was_fallback) = ctx.resolve_theme(args.theme.clone(), args.fallback)?;

    let generator = ctx.resolve_generator(&args.generator)?;
    if !generator.is_installed() {
        anyhow::bail!(
            "Cannot apply theme: application `{}` is not installed in your system.",
            generator.name().cyan().bold()
        );
    }

    if ctx.log.is_detailed() {
        render_header(&theme_name, args.theme.as_deref(), was_fallback);
    }
    println!();

    let orchestrator = ThemeOrchestrator::new(&ctx.paths, &ctx.log);
    let theme_obj: Theme = orchestrator.load_theme(&theme_name, false, true, &ctx.state)?;

    ensure_generator_health(generator, &theme_obj, ctx, false)?;

    let gen_color = generator.generator_type().color();
    let mut step = ctx.log.step_with_icon(
        generator.generator_type().icon().color(gen_color),
        &format!(
            "Applying {} to {}",
            theme_obj.name.yellow().bold(),
            generator.name().cyan().bold()
        ),
        true,
    );

    let result = generator.apply(&theme_obj, &ctx.paths, &ctx.templater, &mut step);
    let final_msg: String = if was_fallback {
        format!(
            "Applied theme {} to {} {}",
            theme_obj.name.yellow().bold(),
            generator.name().cyan().bold(),
            format!("(using fallback: {})", theme_name).dimmed()
        )
    } else {
        format!(
            "Applied theme {} to {}",
            theme_obj.name.yellow().bold(),
            generator.name().cyan().bold()
        )
    };

    step.done_with(&final_msg);
    result?;

    if !ctx.log.is_detailed() {
        println!();
    }

    Ok(())
}

/// Helper function to ensure generator health
fn ensure_generator_health(
    g: &dyn Generator,
    theme: &Theme,
    ctx: &IrisContext,
    is_last: bool,
) -> anyhow::Result<()> {
    let status = g.health_check(&ctx.paths, &theme.name);

    if !status.is_ok() {
        let mut fix_step = ctx.log.step(
            &format!(
                "Fixing `{}` problem: {}",
                g.name().cyan(),
                status.message().dimmed()
            ),
            is_last,
        );

        let _ = g.fix(&status, theme, &ctx.paths, &ctx.templater, &mut fix_step)?;
        fix_step.done_with(&format!("Repaired `{}` successfully", g.name().cyan()));
    }

    Ok(())
}

/// Helper function to render header for apply
fn render_header(theme: &str, original: Option<&str>, is_fallback: bool) {
    println!(
        "\n{}  {} {}",
        "󰷉".yellow().bold(),
        "Standalone apply:".bold(),
        theme.magenta()
    );

    if is_fallback {
        println!(
            "{}  Using fallback: {} because `{}` was unavailable",
            "󰁯".blue(),
            theme.green().bold(),
            original.unwrap_or("current").dimmed()
        );
    }
}
