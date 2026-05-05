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

    pub fn is_error(&self) -> bool {
        matches!(self, HealthStatus::Error { .. })
    }

    pub fn message(&self) -> String {
        match self {
            HealthStatus::Ok => "healthy".to_string(),
            HealthStatus::Warning(msg) => msg.clone(),
            HealthStatus::Error { message, .. } => message.clone(),
        }
    }

    pub fn icon(&self) -> String {
        match self {
            HealthStatus::Ok => "󰄬".green().to_string(),
            HealthStatus::Warning(_) => "󱈸".yellow().to_string(),
            HealthStatus::Error { .. } => "󰅚".red().to_string(),
        }
    }

    pub fn error(msg: impl Into<String>, hint: Option<impl Into<String>>) -> Self {
        Self::Error {
            message: msg.into(),
            fix_hint: hint.map(|h| h.into()),
        }
    }
}
