use crate::core::IrisContext;
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
pub fn exec(ctx: &IrisContext) -> anyhow::Result<()> {
    println!("\n  {}\n", "󰓦  Iris System Health".bright_red().bold());

    for generator in &ctx.registry.all() {
        let status = generator.health_check(ctx);
        let name = generator.name();

        match status {
            HealthStatus::Ok => {
                println!(
                    "  {}  {:<12}  {}",
                    "󰄬".green(),
                    name.bold(),
                    "healthy".dimmed()
                );
            }
            HealthStatus::Warning(msg) => {
                println!("  {}  {:<12}  {}", "󱈸".yellow(), name.bold(), msg.yellow());
            }
            HealthStatus::Error { message, fix_hint } => {
                println!("  {}  {:<12}  {}", "󰅚".red(), name.bold(), message.red());
                if let Some(hint) = fix_hint {
                    println!("      {}  {}", "󰋽".blue(), hint.dimmed());
                }
            }
        }
    }

    println!("\n  {}  {}", "󰚥".dimmed(), "Check complete".dimmed());
    Ok(())
}
