use colored::Colorize;

/// Module health status
#[derive(Debug, Clone, PartialEq, Eq)]
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

    pub fn icon(&self) -> String {
        match self {
            HealthStatus::Ok => "󰄬".green().to_string(),
            HealthStatus::Warning(_) => "󱈸".yellow().to_string(),
            HealthStatus::Error { .. } => "󰅚".red().to_string(),
        }
    }
}
