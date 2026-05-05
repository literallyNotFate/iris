use crate::{
    core::IrisContext,
    log::Reporter,
    models::{HealthStatus, Palette},
};
use colored::Colorize;
use std::io::{self, Write};

/// Handle application health command with fix option
pub fn exec(fix: bool, ctx: &mut IrisContext) -> anyhow::Result<()> {
    use colored::*;
    println!("\n{}\n", "󰓦  Iris System Health".bright_red().bold());

    let mut issues_found = false;
    let mut palette = None;

    let mut healthy = Vec::new();
    let mut errors = Vec::new();

    for generator in ctx.registry.all() {
        if !ctx.state.is_enabled(generator.name()) {
            continue;
        }

        let status: HealthStatus = generator.health_check(&ctx.paths, &ctx.state.current_theme);
        if status.is_ok() {
            healthy.push((generator, status));
        } else {
            issues_found = true;
            errors.push((generator, status));
        }
    }

    if fix {
        if !errors.is_empty() {
            println!("  {}", "[!] Issues Detected:".yellow().bold());
            for (i, (generator, status)) in errors.iter().enumerate() {
                let is_last = i == errors.len() - 1;
                render_status_line(generator.name(), &status, is_last, &ctx.log);
            }

            println!();
        }

        if !errors.is_empty() {
            if palette.is_none() {
                palette = Some(Palette::fetch(
                    &ctx.state.current_theme,
                    false,
                    true,
                    &ctx.paths,
                    &ctx.state,
                    &Reporter::quiet(),
                )?);
            }

            if let Some(p) = &palette {
                println!("  {}", "Applying fixes:".dimmed());

                for (i, (generator, status)) in errors.iter().enumerate() {
                    if i > 0 {
                        println!();
                    }

                    let name = generator.name().to_string();
                    print!("    {} Fixing {} ... ", "-> ".dimmed(), name.bold());
                    let _ = io::stdout().flush();

                    let mut task = ctx.log.as_task();

                    generator.fix(&status, p, &ctx.paths, &ctx.templater, &mut task)?;
                }

                println!();
            }
        }
    } else {
        if !errors.is_empty() {
            println!("  {}", "[!] Issues Detected:".yellow().bold());
            for (i, (generator, status)) in errors.iter().enumerate() {
                let is_last = i == errors.len() - 1;
                render_status_line(generator.name(), &status, is_last, &ctx.log);
            }
            println!();
        }

        if !healthy.is_empty() {
            let header = if issues_found {
                "[✓] All Good:".green().bold()
            } else {
                "[✓] All Systems Operational:".green().bold()
            };

            println!("  {}", header);
            for (i, (generator, status)) in healthy.iter().enumerate() {
                let is_last = i == healthy.len() - 1;
                render_status_line(generator.name(), &status, is_last, &ctx.log);
            }
            println!();
        }
    }

    if issues_found && !fix {
        ctx.log.info(&format!(
            "Run `{}` to resolve issues automatically",
            "iris health --fix".cyan().bold()
        ));
    } else if !issues_found {
        ctx.log.success("System is healthy!");
    } else {
        ctx.log.success("All issues have been resolved!");
    }

    println!();
    Ok(())
}

fn render_status_line(name: &str, status: &HealthStatus, is_last: bool, log: &Reporter) {
    let branch = if is_last { "└─ " } else { "├─ " };

    print!(
        "{}   {}{} {:<12} ",
        log.gutter,
        branch.dimmed(),
        status.icon(),
        name.bold()
    );

    match status {
        HealthStatus::Ok => {
            println!("{}", "healthy".blue().bold());
        }
        HealthStatus::Warning(msg) => {
            println!("{}", msg.yellow());
        }
        HealthStatus::Error { message, fix_hint } => {
            println!("{}", message.red());

            if let Some(hint) = fix_hint {
                let indent = if is_last { "   " } else { "│  " };
                println!(
                    "{}   {}{} {}",
                    log.gutter,
                    indent.dimmed(),
                    "󰋽".blue(),
                    hint.dimmed()
                );
            }
        }
    }
}
