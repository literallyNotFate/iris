use crate::ui::task::Task;
use colored::*;
use std::io::{self, Write};

/// Manages global UI state and creates tasks
#[derive(Clone)]
pub struct Logger {
    pub quiet: bool,
}

impl Logger {
    /// Creates a new Logger instance
    pub fn new() -> Self {
        Self { quiet: false }
    }

    /// Returns new instance of a logger with quiet mode turned on
    pub fn quiet() -> Self {
        Self { quiet: true }
    }

    /// Generates formatting prefixes based on indent level and activity state
    pub fn get_prefix(&self, level: u8, active: bool) -> String {
        let indent = "  ".repeat(level as usize);
        if active {
            format!("{}{}", indent, "❯".blue().bold())
        } else {
            let branch = if level == 0 {
                "".to_string()
            } else {
                format!("{} ", "├─".blue())
            };
            format!("{}{}", indent, branch)
        }
    }

    /// Starts a new timed task
    ///
    /// If `quiet` is enabled, it prints a compact start message
    pub fn step(&self, message: &str, level: u8) -> Task {
        print!("{}", self.build_step_output(message, level));
        let _ = io::stdout().flush();
        Task::new(message.to_string(), level, self.quiet)
    }

    /// Logs an informative message (Level 2).
    pub fn info(&self, message: &str) {
        if !self.quiet {
            print!("{}", self.build_info_output(message));
        }
    }

    /// Logs a simple success message without a timer
    pub fn success(&self, message: &str, level: u8) {
        print!("{}", self.build_success_output(message, level));
    }

    /// Logs a warning message
    pub fn warn(&self, message: &str, level: u8) {
        print!("{}", self.build_warning_output(message, level));
    }

    /// Logs an error message
    pub fn error(&self, message: &str, level: u8) {
        print!("{}", self.build_error_output(message, level));
    }
}

/// Build API (for testing)
impl Logger {
    /// Build string for step message
    fn build_step_output(&self, message: &str, level: u8) -> String {
        if self.quiet {
            format!("   {} {}... ", "❯".blue().bold(), message.white().bold())
        } else {
            format!("{} {}\n", self.get_prefix(level, true), message.white())
        }
    }

    /// Build string for info output message
    fn build_info_output(&self, message: &str) -> String {
        format!("{} {}\n", self.get_prefix(2, false), message.dimmed())
    }

    /// Build string for success output message
    fn build_success_output(&self, message: &str, level: u8) -> String {
        let icon = if level == 0 {
            "✔".green().bold()
        } else {
            "├─".blue()
        };
        let indent = "  ".repeat(level as usize);
        format!("{} {} {}\n", indent, icon, message)
    }

    /// Build string for warning output message
    fn build_warning_output(&self, message: &str, level: u8) -> String {
        let icon = "!".yellow().bold();
        format!(
            "{}{} {}\n",
            self.get_prefix(level, false),
            icon,
            message.yellow()
        )
    }

    /// Build string for error output message
    fn build_error_output(&self, message: &str, level: u8) -> String {
        let icon = "✘".red().bold();
        format!(
            "{}{} {}\n",
            self.get_prefix(level, false),
            icon,
            message.red()
        )
    }
}

/// Unit-tests for logger
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_create_logger_in_quiet_mode() {
        let logger: Logger = Logger::new();
        let quiet_logger: Logger = Logger::quiet();

        assert_eq!(logger.quiet, false);
        assert_eq!(quiet_logger.quiet, true);
    }

    #[test]
    fn should_handle_logger_prefix_logic() {
        let logger: Logger = Logger::new();

        assert_eq!(logger.get_prefix(0, true).contains("❯"), true);
        assert_eq!(logger.get_prefix(0, false), "");

        let active: String = logger.get_prefix(1, true);
        let inactive: String = logger.get_prefix(1, false);

        assert!(active.contains("  "));
        assert!(active.contains("❯"));
        assert!(inactive.contains("├─"));
    }

    #[test]
    fn should_build_logger_success_messages() {
        let logger: Logger = Logger::new();

        let out_0: String = logger.build_success_output("Root", 0);
        assert!(out_0.contains("✔"));
        assert!(out_0.contains("Root"));

        let out_1: String = logger.build_success_output("Nested", 1);
        assert!(out_1.contains("├─"));
        assert!(out_1.contains("Nested"));
    }

    #[test]
    fn should_build_logger_warn_and_error_messages() {
        let logger: Logger = Logger::new();

        let warn_out: String = logger.build_warning_output("Warning", 0);
        assert!(warn_out.contains("!"));
        assert!(warn_out.contains("Warning"));

        let err_out: String = logger.build_error_output("Error", 0);
        assert!(err_out.contains("✘"));
        assert!(err_out.contains("Error"));
    }

    #[test]
    fn should_build_logger_info_messages() {
        let output: String = Logger::new().build_info_output("Visible");
        assert!(output.contains("Visible"));
        assert!(output.contains("•") || output.contains("├─"));
    }

    #[test]
    fn should_build_step_output_normal() {
        let logger: Logger = Logger::new();
        let output: String = logger.build_step_output("Test", 0);

        assert!(!output.contains("..."));
        assert!(output.contains("❯"));
    }

    #[test]
    fn should_build_step_output_quiet() {
        let logger: Logger = Logger::quiet();
        let output: String = logger.build_step_output("Test", 0);

        assert!(output.contains("..."));
    }
}
