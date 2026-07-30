use crate::{
    core::IrisContext,
    log::Logger,
    models::{HealthStatus, health::IssueSeverity},
    service::ThemeService,
};
use colored::*;
use std::io::{self, Write};

/// Handle application health command with fix option
pub fn exec(fix: bool, ctx: &mut IrisContext) -> anyhow::Result<()> {
    println!();

    if ctx.log.is_detailed() {
        println!("{}\n", "󰓦  Iris System Health".bright_red().bold());
    }

    let theme: &String = &ctx.state.theme.current_theme;
    let (healthy, errors) = ctx.registry.check_all(&ctx.state, &ctx.paths, theme);
    let issues_found = !errors.is_empty();

    if fix {
        if issues_found {
            if ctx.log.is_detailed() {
                println!("  {}", "[!] Issues Detected:".yellow().bold());
                for (i, (generator, status)) in errors.iter().enumerate() {
                    let is_last = i == errors.len() - 1;
                    render_status_line(generator.name(), status, is_last, &ctx.log);
                }
                println!();
                println!("  {}", "Applying fixes:".dimmed());
            }

            let quiet_logger: Logger = Logger::silent();
            let service = ThemeService::new(&ctx.paths, &quiet_logger);
            let theme = service.load_theme(theme, false, true, &ctx.state)?;
            let engine = ctx.engine(&theme);

            for (i, (generator, status)) in errors.iter().enumerate() {
                if ctx.log.is_detailed() {
                    if i > 0 {
                        println!();
                    }

                    print!(
                        "   {} Fixing {} ... ",
                        "-> ".dimmed(),
                        generator.name().bold()
                    );
                    let _ = io::stdout().flush();
                }

                let mut activity = ctx.log.activity();
                engine.execute_fix(*generator, &status, &mut activity)?;
            }

            if ctx.log.is_detailed() {
                println!();
            }
        }
    } else {
        if ctx.log.is_detailed() {
            if issues_found {
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
        } else if !ctx.log.is_detailed() && issues_found {
            println!(
                "{}  Found {} issue(s), operational: {}/{}",
                "󰀦".yellow().bold(),
                errors.len().to_string().red().bold(),
                healthy.len(),
                healthy.len() + errors.len()
            );
        }
    }

    if ctx.log.is_detailed() {
        if issues_found && !fix {
            ctx.log.info(&format!(
                "Run `{}` to resolve issues automatically.",
                "iris health --fix".cyan().bold()
            ));
        } else if !issues_found {
            ctx.log.success("System is healthy!");
        } else {
            ctx.log.success("All issues have been resolved!");
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
        HealthStatus::Issue(severity, _, _) => {
            let msg = status.message();
            match severity {
                IssueSeverity::Warning => println!("{}", msg.yellow()),
                IssueSeverity::Error => println!("{}", msg.red()),
            }

            if let Some(hint) = status.hint() {
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
