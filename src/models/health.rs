use colored::Colorize;
use std::{fmt, path::Path};

/// Issue severite
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IssueSeverity {
    /// There are some issues but not severe ones, can work still (e.g uses old theme)
    Warning,
    /// Critical error that needs fix (e.g config file missing)
    Error,
}

/// Generator issue
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Issue {
    BinaryNotFound,
    ConfigMissing,
    ImportMissing,
    SymlinkInvalid,
    CacheMismatch,
    EnvMismatch,
    CacheMissing,
    BlockMissing,
    MarkerMissing,
}

/// Module health status.
/// Used for config diagnostics, file existing checks, automatic fixes via CLI
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthStatus {
    /// Module is ready to go
    Ok,
    /// There is some issue with certain severity and fix hint message
    Issue(IssueSeverity, Issue, Option<String>),
}

impl HealthStatus {
    /// Warning status without hint
    pub fn warn(issue: Issue) -> Self {
        Self::Issue(IssueSeverity::Warning, issue, None)
    }

    /// Warning status with hint
    pub fn warn_with_hint(issue: Issue, hint: impl Into<String>) -> Self {
        Self::Issue(IssueSeverity::Warning, issue, Some(hint.into()))
    }

    /// Error status without hint
    pub fn error(issue: Issue) -> Self {
        Self::Issue(IssueSeverity::Error, issue, None)
    }

    /// Error status with hint
    pub fn error_with_hint(issue: Issue, hint: impl Into<String>) -> Self {
        Self::Issue(IssueSeverity::Error, issue, Some(hint.into()))
    }

    /// Returns true if status - Ok
    pub fn is_ok(&self) -> bool {
        matches!(self, HealthStatus::Ok)
    }

    /// Returns true if status - Error
    pub fn is_error(&self) -> bool {
        matches!(self, HealthStatus::Issue(IssueSeverity::Error, ..))
    }

    /// Returns true if status - Warning
    pub fn is_warning(&self) -> bool {
        matches!(self, HealthStatus::Issue(IssueSeverity::Warning, ..))
    }

    /// Checks whether status message contains certain text (case insensitive)
    pub fn contains(&self, text: &str) -> bool {
        self.message().to_lowercase().contains(&text.to_lowercase())
    }

    /// Returns current issue message
    pub fn message(&self) -> String {
        match self {
            HealthStatus::Ok => "healthy".to_string(),
            HealthStatus::Issue(_, issue, _) => issue.to_string(),
        }
    }

    /// Returns colored status icon (Nerd Font)
    pub fn icon(&self) -> String {
        match self {
            HealthStatus::Ok => "󰄬".green().to_string(),
            HealthStatus::Issue(IssueSeverity::Warning, ..) => "󱈸".yellow().to_string(),
            HealthStatus::Issue(IssueSeverity::Error, ..) => "󰅚".red().to_string(),
        }
    }

    /// Returns fix_hint if current status is error
    pub fn hint(&self) -> Option<&String> {
        if let HealthStatus::Issue(_, _, Some(hint)) = self {
            Some(hint)
        } else {
            None
        }
    }

    /// Checks file existence. If no file found - returns Error
    pub fn check_file(path: &Path, issue: Issue) -> Self {
        if !path.exists() {
            Self::error_with_hint(issue, format!("Path not found: {}.", path.display()))
        } else {
            Self::Ok
        }
    }

    /// Checks if a path is a symlink.
    /// If no file found - Error. If file found but its not a symlink - Error
    pub fn check_symlink(path: &Path, issue: Issue) -> Self {
        if !path.exists() {
            return Self::check_file(path, issue);
        }

        if !path.is_symlink() {
            return Self::error_with_hint(issue, format!("Not a symlink: {}.", path.display()));
        }

        Self::Ok
    }
}

/// Display trait realisation for terminal: <icon> <message>
impl fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let color = match self {
            HealthStatus::Ok => "green",
            HealthStatus::Issue(IssueSeverity::Warning, ..) => "yellow",
            HealthStatus::Issue(IssueSeverity::Error, ..) => "red",
        };
        write!(f, "{} {}", self.icon(), self.message().color(color))
    }
}

/// Display trait realisation for issue
impl fmt::Display for Issue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            Issue::BinaryNotFound => "Binary not found",
            Issue::ConfigMissing => "Configuration file missing",
            Issue::ImportMissing => "Theme not imported",
            Issue::SymlinkInvalid => "Invalid symlink",
            Issue::CacheMismatch => "Cache mismatch",
            Issue::EnvMismatch => "Environment variable mismatch",
            Issue::CacheMissing => "Cache file missing",
            Issue::BlockMissing => "Theme block missing",
            Issue::MarkerMissing => "Theme marker missing",
        };
        write!(f, "{}", msg)
    }
}

/// Allows converting Result to HealthStatus
impl<T, E: fmt::Display> From<Result<T, E>> for HealthStatus {
    fn from(res: Result<T, E>) -> Self {
        match res {
            Ok(_) => HealthStatus::Ok,
            Err(e) => HealthStatus::error_with_hint(Issue::ConfigMissing, e.to_string()),
        }
    }
}

/// Unit-tests for health
#[cfg(test)]
mod tests {
    use super::*;
    use tempdir::TempDir;

    #[test]
    fn should_correctly_identify_types_via_predicates() {
        let ok = HealthStatus::Ok;
        let warn = HealthStatus::warn(Issue::CacheMismatch);
        let err = HealthStatus::error_with_hint(Issue::ConfigMissing, "Check path");

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
        let warn = HealthStatus::warn(Issue::CacheMismatch);
        let err = HealthStatus::error_with_hint(Issue::ConfigMissing, "Run fix");

        assert_eq!(ok.message(), "healthy");
        assert_eq!(warn.message(), "Cache mismatch");
        assert_eq!(err.message(), "Configuration file missing");

        assert!(ok.hint().is_none());
        assert!(warn.hint().is_none());
        assert_eq!(err.hint(), Some(&"Run fix".to_string()));
    }

    #[test]
    fn should_perform_case_insensitive_contains_matching() {
        let warn = HealthStatus::warn(Issue::CacheMismatch);
        let err = HealthStatus::error_with_hint(Issue::ConfigMissing, "Hint");

        assert!(warn.contains("cache"));
        assert!(warn.contains("MISMATCH"));
        assert!(!warn.contains("tmux"));

        assert!(err.contains("config"));
        assert!(err.contains("missing"));
        assert!(!err.contains("healthy"));
    }

    #[test]
    fn should_check_file_existence() {
        let tmp_dir: TempDir = TempDir::new("health_test").expect("Cannot create temp folder");
        let file_path = tmp_dir.path().join("config.toml");

        let status = HealthStatus::check_file(&file_path, Issue::ConfigMissing);
        assert!(status.is_error());
        assert!(status.contains("missing"));
        assert!(status.hint().unwrap().contains("config.toml"));

        std::fs::write(&file_path, "content").unwrap();
        let status = HealthStatus::check_file(&file_path, Issue::ConfigMissing);
        assert!(status.is_ok());
    }

    #[test]
    fn should_check_symlinks_correctly() {
        let tmp_dir: TempDir = TempDir::new("health_test").expect("Cannot create temp folder");
        let root = tmp_dir.path();

        let target_file = root.join("target.txt");
        let link_path = root.join("symlink.txt");
        let regular_file = root.join("regular.txt");

        std::fs::write(&target_file, "target").unwrap();
        std::fs::write(&regular_file, "regular").unwrap();

        let status = HealthStatus::check_symlink(&link_path, Issue::SymlinkInvalid);
        assert!(status.is_error());
        assert!(status.contains("invalid symlink"));

        let status = HealthStatus::check_symlink(&regular_file, Issue::SymlinkInvalid);
        assert!(status.is_error());
        assert!(status.contains("invalid symlink"));

        #[cfg(unix)]
        std::os::unix::fs::symlink(&target_file, &link_path).unwrap();
        let status = HealthStatus::check_symlink(&link_path, Issue::SymlinkInvalid);
        assert!(status.is_ok());
    }

    #[test]
    fn should_format_display_with_colors_and_icons() {
        let ok = HealthStatus::Ok;
        let warn = HealthStatus::warn(Issue::CacheMismatch);
        let err = HealthStatus::error_with_hint(Issue::ConfigMissing, "Hint");

        assert!(format!("{ok}").contains("󰄬"));
        assert!(format!("{ok}").contains("healthy"));

        assert!(format!("{warn}").contains("󱈸"));
        assert!(format!("{warn}").contains("Cache mismatch"));

        assert!(format!("{err}").contains("󰅚"));
        assert!(format!("{err}").contains("Configuration file missing"));
    }
}
