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
    pub fn new(quiet: bool) -> Self {
        Self { quiet }
    }

    /// Returns new instance of a logger with quiet mode turned on
    pub fn as_quiet(&self) -> Self {
        let mut quiet_logger = self.clone();
        quiet_logger.quiet = true;
        quiet_logger
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
        if self.quiet {
            print!("   {} {}... ", "❯".blue().bold(), message.white().bold());
            println!();
            let _ = io::stdout().flush();
        } else {
            println!("{} {}", self.get_prefix(level, true), message.white());
        }

        Task::new(message.to_string(), level, self.quiet)
    }

    /// Logs an informative message (Level 2).
    pub fn info(&self, message: &str) {
        if !self.quiet {
            println!("{} {}", self.get_prefix(2, false), message.dimmed());
        }
    }

    /// Logs a simple success message without a timer
    pub fn success(&self, message: &str, level: u8) {
        let icon = if level == 0 {
            "✔".green().bold()
        } else {
            "├─".blue()
        };

        let indent = "  ".repeat(level as usize);
        println!("{} {} {}", indent, icon, message);
    }

    /// Logs a warning message
    pub fn warn(&self, message: &str, level: u8) {
        let icon = "!".yellow().bold();

        println!(
            "{}{} {}",
            self.get_prefix(level, false),
            icon,
            message.yellow()
        );
    }

    /// Logs an error message
    pub fn error(&self, message: &str, level: u8) {
        let icon = "✘".red().bold();

        println!(
            "{}{} {}",
            self.get_prefix(level, false),
            icon,
            message.red()
        );
    }
}
