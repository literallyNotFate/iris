use colored::Colorize;
use std::{fmt, path::Path};

/// Module health status.
/// Used for config diagnostics, file existing checks, automatic fixes via CLI
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthStatus {
    /// Module is ready to go
    Ok,
    /// There are some issues but not severe ones, can work still (e.g uses old theme)
    Warning(String),
    /// Critical error that needs fix (e.g config file missing)
    Error {
        message: String,
        fix_hint: Option<String>,
    },
}

impl HealthStatus {
    /// Creates new error status with message and optional hint
    pub fn error(msg: impl Into<String>, hint: Option<impl Into<String>>) -> Self {
        Self::Error {
            message: msg.into(),
            fix_hint: hint.map(|h| h.into()),
        }
    }

    /// Returns true if status - Ok
    pub fn is_ok(&self) -> bool {
        matches!(self, HealthStatus::Ok)
    }

    /// Returns true if status - Error
    pub fn is_error(&self) -> bool {
        matches!(self, HealthStatus::Error { .. })
    }

    /// Returns true if status - Warning
    pub fn is_warning(&self) -> bool {
        matches!(self, HealthStatus::Warning(_))
    }

    /// Checks whether status message contains certain text (case insensitive)
    pub fn contains(&self, text: &str) -> bool {
        let text = text.to_lowercase();
        match self {
            HealthStatus::Ok => false,
            HealthStatus::Warning(msg) => msg.to_lowercase().contains(&text),
            HealthStatus::Error { message, .. } => message.to_lowercase().contains(&text),
        }
    }

    /// Returns current status message
    pub fn message(&self) -> String {
        match self {
            HealthStatus::Ok => "healthy".to_string(),
            HealthStatus::Warning(msg) => msg.clone(),
            HealthStatus::Error { message, .. } => message.clone(),
        }
    }

    /// Returns colored status icon (Nerd Font)
    pub fn icon(&self) -> String {
        match self {
            HealthStatus::Ok => "󰄬".green().to_string(),
            HealthStatus::Warning(_) => "󱈸".yellow().to_string(),
            HealthStatus::Error { .. } => "󰅚".red().to_string(),
        }
    }

    /// Returns fix_hint if current status is error
    pub fn hint(&self) -> Option<&String> {
        if let HealthStatus::Error { fix_hint, .. } = self {
            fix_hint.as_ref()
        } else {
            None
        }
    }

    /// Checks file existence. If no file found - returns Error
    pub fn check_file(path: &Path, label: &str) -> Self {
        if !path.exists() {
            return Self::error(
                format!("{} not found", label),
                Some(format!("Expected: {}", path.display())),
            );
        }

        Self::Ok
    }

    /// Checks if a path is a symlink.
    /// If no file found - Error. If file found but its not a symlink - Error
    pub fn check_symlink(path: &Path, label: &str) -> Self {
        if !path.exists() {
            return Self::check_file(path, label);
        }

        if !path.is_symlink() {
            return Self::error(
                format!("{} is not a symlink", label),
                Some(format!("Target: {}", path.display())),
            );
        }

        Self::Ok
    }
}

/// Display trait realisation for terminal: <icon> <message>
impl fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HealthStatus::Ok => write!(f, "{} {}", self.icon(), self.message().green()),
            HealthStatus::Warning(msg) => write!(f, "{} {}", self.icon(), msg.yellow()),
            HealthStatus::Error { message, .. } => write!(f, "{} {}", self.icon(), message.red()),
        }
    }
}

/// Allows converting Result to HealthStatus
impl<T, E: fmt::Display> From<Result<T, E>> for HealthStatus {
    fn from(res: Result<T, E>) -> Self {
        match res {
            Ok(_) => HealthStatus::Ok,
            Err(e) => HealthStatus::Warning(e.to_string()),
        }
    }
}

/// Unit-tests for health
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempdir::TempDir;

    #[test]
    fn should_correctly_identify_types_via_predicates() {
        let ok = HealthStatus::Ok;
        let warn = HealthStatus::Warning("Mismatched theme".into());
        let err = HealthStatus::error("File missing", Some("Check path"));

        assert!(ok.is_ok());
        assert!(!warn.is_ok());
        assert!(!err.is_ok());

        assert!(warn.is_warning());
        assert!(!ok.is_warning());
        assert!(!err.is_warning());

        assert!(err.is_error());
        assert!(!ok.is_error());
        assert!(!warn.is_error());
    }

    #[test]
    fn should_extract_messages_and_hints() {
        let ok = HealthStatus::Ok;
        let warn = HealthStatus::Warning("Some warning".into());
        let err = HealthStatus::error("Critical issue", Some("Run fix"));

        assert_eq!(ok.message(), "healthy");
        assert_eq!(warn.message(), "Some warning");
        assert_eq!(err.message(), "Critical issue");

        assert!(ok.hint().is_none());
        assert!(warn.hint().is_none());
        assert_eq!(err.hint(), Some(&"Run fix".to_string()));
    }

    #[test]
    fn should_perform_case_insensitive_contains_matching() {
        let warn = HealthStatus::Warning("Starship config is broken".into());
        let err = HealthStatus::error("BTOP.CONF is missing", Some("Hint"));

        assert!(warn.contains("starship"));
        assert!(warn.contains("BROKEN"));
        assert!(!warn.contains("tmux"));

        assert!(err.contains("btop"));
        assert!(err.contains("missing"));
        assert!(!err.contains("healthy"));

        assert!(!HealthStatus::Ok.contains("healthy"));
    }

    #[test]
    fn should_check_file_existence() {
        let tmp_dir: TempDir = TempDir::new("health_test").unwrap();
        let file_path = tmp_dir.path().join("config.toml");

        let status = HealthStatus::check_file(&file_path, "Config file");
        assert!(status.is_error());
        assert!(status.contains("not found"));
        assert!(status.hint().unwrap().contains("config.toml"));

        fs::write(&file_path, "content").unwrap();
        let status = HealthStatus::check_file(&file_path, "Config file");
        assert!(status.is_ok());
    }

    #[test]
    fn should_check_symlinks_correctly() {
        let tmp_dir: TempDir = TempDir::new("health_test").unwrap();
        let root = tmp_dir.path();

        let target_file = root.join("target.txt");
        let link_path = root.join("symlink.txt");
        let regular_file = root.join("regular.txt");

        fs::write(&target_file, "target").unwrap();
        fs::write(&regular_file, "regular").unwrap();

        let status = HealthStatus::check_symlink(&link_path, "Theme link");
        assert!(status.is_error());
        assert!(status.contains("not found"));

        let status = HealthStatus::check_symlink(&regular_file, "Theme link");
        assert!(status.is_error());
        assert!(status.contains("not a symlink"));

        #[cfg(unix)]
        std::os::unix::fs::symlink(&target_file, &link_path).unwrap();
        let status = HealthStatus::check_symlink(&link_path, "Theme link");
        assert!(status.is_ok());
    }

    #[test]
    fn should_convert_from_result() {
        let res_ok: Result<&str, &str> = Ok("success");
        let status_ok: HealthStatus = res_ok.into();
        assert!(status_ok.is_ok());

        let res_err: Result<&str, String> = Err("IO operational failure".to_string());
        let status_err: HealthStatus = res_err.into();
        assert!(status_err.is_warning());
        assert!(status_err.contains("operational failure"));
    }

    #[test]
    fn should_format_display_with_colors_and_icons() {
        let ok = HealthStatus::Ok;
        let warn = HealthStatus::Warning("Fix me".into());
        let err = HealthStatus::error("Fail", Some("Hint"));

        assert!(format!("{ok}").contains("󰄬"));
        assert!(format!("{ok}").contains("healthy"));

        assert!(format!("{warn}").contains("󱈸"));
        assert!(format!("{warn}").contains("Fix me"));

        assert!(format!("{err}").contains("󰅚"));
        assert!(format!("{err}").contains("Fail"));
    }
}
