use crate::{
    core::{IrisContext, ThemeOrchestrator},
    log::{Logger, LoggingVerbosity},
    models::{HealthStatus, Theme},
};
use colored::*;
use std::io::{self, Write};

/// Handle application health command with fix option
pub fn exec(fix: bool, ctx: &mut IrisContext) -> anyhow::Result<()> {
    println!();
    let verbosity: LoggingVerbosity = ctx.log.verbosity;

    if ctx.log.is_detailed() {
        println!("{}\n", "󰓦  Iris System Health".bright_red().bold());
    }

    let mut issues_found = false;
    let mut theme_obj: Option<Theme> = None;

    let mut healthy = Vec::new();
    let mut errors = Vec::new();

    for generator in ctx.registry.all() {
        if !ctx.state.is_enabled(generator.name()) {
            continue;
        }

        let status: HealthStatus =
            generator.health_check(&ctx.paths, &ctx.state.theme.current_theme);
        if status.is_ok() {
            healthy.push((generator, status));
        } else {
            issues_found = true;
            errors.push((generator, status));
        }
    }

    if fix {
        if !errors.is_empty() {
            if ctx.log.is_detailed() {
                println!("  {}", "[!] Issues Detected:".yellow().bold());
                for (i, (generator, status)) in errors.iter().enumerate() {
                    let is_last = i == errors.len() - 1;
                    render_status_line(generator.name(), status, is_last, &ctx.log);
                }
                println!();
                println!("  {}", "Applying fixes:".dimmed());
            }

            if theme_obj.is_none() {
                let quiet_logger: Logger = Logger::silent();
                let orchestrator = ThemeOrchestrator::new(&ctx.paths, &quiet_logger);
                theme_obj = Some(orchestrator.load_theme(
                    &ctx.state.theme.current_theme,
                    false,
                    true,
                    &ctx.state,
                )?);
            }

            if let Some(theme) = &theme_obj {
                for (i, (generator, status)) in errors.iter().enumerate() {
                    if ctx.log.is_detailed() {
                        if i > 0 {
                            println!();
                        }

                        print!(
                            "    {} Fixing {} ... ",
                            "-> ".dimmed(),
                            generator.name().bold()
                        );
                        let _ = io::stdout().flush();
                    }

                    let mut task = ctx.log.as_task();
                    generator.fix(status, theme, &ctx.paths, &ctx.templater, &mut task)?;
                }

                if ctx.log.is_detailed() {
                    println!();
                }
            }
        }
    } else {
        if ctx.log.is_detailed() {
            if !errors.is_empty() {
                println!("  {}", "[!] Issues Detected:".yellow().bold());
                for (i, (generator, status)) in errors.iter().enumerate() {
                    let is_last = i == errors.len() - 1;
                    render_status_line(generator.name(), status, is_last, &ctx.log);
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
                    render_status_line(generator.name(), status, is_last, &ctx.log);
                }
                println!();
            }
        } else if verbosity == LoggingVerbosity::Minimal && issues_found {
            println!(
                "{}  Found {} issue(s), operational: {}/{}",
                "󰀦".yellow().bold(),
                errors.len().to_string().red().bold(),
                healthy.len(),
                healthy.len() + errors.len()
            );
        }
    }

    if verbosity != LoggingVerbosity::Silent {
        if issues_found && !fix {
            if ctx.log.is_detailed() {
                ctx.log.info(&format!(
                    "Run `{}` to resolve issues automatically",
                    "iris health --fix".cyan().bold()
                ));
            } else {
                println!(
                    "   Run `{}` to resolve automatically.",
                    "iris health --fix".cyan().italic()
                );
            }
        } else if !issues_found {
            if ctx.log.is_detailed() {
                ctx.log.success("System is healthy!");
            } else {
                println!(
                    "{}  {}",
                    "✓".green().bold(),
                    "System is healthy!".white().bold()
                );
            }
        } else {
            if ctx.log.is_detailed() {
                ctx.log.success("All issues have been resolved!");
            } else {
                println!(
                    "{}  {}",
                    "✓".green().bold(),
                    "All issues have been resolved!".white().bold()
                );
            }
        }

        println!();
    }

    Ok(())
}

fn render_status_line(name: &str, status: &HealthStatus, is_last: bool, log: &Logger) {
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
