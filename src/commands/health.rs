use crate::{core::IrisContext, models::Palette, ui::Logger};
use colored::Colorize;

/// Module health status
#[derive(Debug)]
pub enum HealthStatus {
    Ok,
    Warning(String),
    Error {
        message: String,
        fix_hint: Option<String>,
    },
}

impl HealthStatus {
    pub fn is_ok(&self) -> bool {
        matches!(self, HealthStatus::Ok)
    }
}

/// Handle application health command
pub fn exec(fix: bool, ctx: &IrisContext) -> anyhow::Result<()> {
    println!("\n  {}\n", "󰓦  Iris System Health".bright_red().bold());

    let mut issues_found: bool = false;
    let palette: Option<Palette> = if fix {
        Some(Palette::fetch(
            &ctx.state.current_theme,
            &Logger::new(true),
        )?)
    } else {
        None
    };

    for generator in &ctx.registry.all() {
        if !ctx.state.is_enabled(generator.name()) {
            continue;
        }

        let status: HealthStatus = generator.health_check(ctx);
        let name: &str = generator.name();

        match &status {
            HealthStatus::Ok => {
                println!(
                    "  {}  {:<12}  {}",
                    "󰄬".green(),
                    name.bold(),
                    "healthy".dimmed()
                );
            }
            _ => {
                issues_found = true;
                match &status {
                    HealthStatus::Warning(msg) => {
                        println!("  {}  {:<12}  {}", "󱈸".yellow(), name.bold(), msg.yellow());
                    }
                    HealthStatus::Error { message, fix_hint } => {
                        println!("  {}  {:<12}  {}", "󰅚".red(), name.bold(), message.red());
                        if let Some(hint) = fix_hint {
                            println!("      {}  {}", "󰋽".blue(), hint.dimmed());
                        }
                    }
                    _ => {}
                }

                if fix {
                    if let Some(ref p) = palette {
                        println!(
                            "    {}  {}",
                            "󰁕".yellow().bold(),
                            "Fixing...".bright_yellow()
                        );

                        match generator.fix(&status, p, ctx) {
                            Ok(_) => {
                                println!(
                                    "\n {} {}\n",
                                    name.bold().green(),
                                    "generator fixed!".green()
                                );
                            }
                            Err(e) => {
                                ctx.log.error(&format!("Failed: {}", e), 1);
                            }
                        }
                    }
                }
            }
        }
    }

    let indent = if !issues_found { "\n" } else { "" };
    println!(
        "{}  {}  {}",
        indent,
        "󰚥".dimmed(),
        "Check complete".dimmed()
    );

    if issues_found && !fix {
        println!(
            "  {}  Run {} to resolve issues automatically.",
            "󰋽".blue(),
            "iris health --fix".cyan().bold()
        );
    }

    Ok(())
}
