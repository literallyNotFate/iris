use crate::{
    core::IrisContext,
    models::{HealthStatus, Palette},
    ui::Logger,
};
use colored::Colorize;

/// Handle application health command with fix option
pub fn exec(fix: bool, ctx: &mut IrisContext) -> anyhow::Result<()> {
    println!("\n  {}\n", "󰓦  Iris System Health".bright_red().bold());

    let mut issues_found: bool = false;
    let palette: Option<Palette> = if fix {
        let p = Palette::fetch(
            &ctx.state.current_theme,
            false,
            &ctx.paths,
            &ctx.state,
            &Logger::quiet(),
        )?;
        Some(p)
    } else {
        None
    };

    for generator in ctx.registry.all() {
        if !ctx.state.is_enabled(generator.name()) {
            continue;
        }

        let status: HealthStatus = generator.health_check(&ctx.paths, &ctx.state.current_theme);
        render_status_line(generator.name(), &status);

        if status.is_ok() {
            continue;
        }

        issues_found = true;
        if fix {
            if let Some(ref p) = palette {
                println!(
                    "      {}  {}",
                    "󰁕".yellow().bold(),
                    "Fixing...".bright_yellow()
                );

                generator.fix(&status, p, &ctx.paths, &ctx.templater, &ctx.log)?;
                println!("      {}  {}", "󰄬".green(), "Fixed successfully!".green());
            }
        }
    }

    println!("\n  {}  {}", "󰚥".dimmed(), "Check complete".dimmed());
    if issues_found && !fix {
        println!(
            "  {}  Run `{}` to resolve issues automatically.",
            "󰋽".blue(),
            "iris health --fix".cyan().bold()
        );
    }

    Ok(())
}

/// Helper function to render status line for generator
fn render_status_line(name: &str, status: &HealthStatus) {
    print!("  {}  {:<12}  ", status.icon(), name.bold());

    match status {
        HealthStatus::Ok => println!("{}", "healthy".dimmed()),
        HealthStatus::Warning(msg) => println!("{}", msg.yellow()),
        HealthStatus::Error { message, fix_hint } => {
            println!("{}", message.red());
            if let Some(hint) = fix_hint {
                println!("      {}  {}", "󰋽".blue(), hint.dimmed());
            }
        }
    }
}
