use crate::{log::Activity, utils};
use colored::*;
use std::{
    io::{self, Write},
    time::Instant,
};

/// Verbose mode for logger
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoggingVerbosity {
    Detailed,
    Minimal,
    Silent,
}

/// A logger responsible for handling the output format, indentation (gutter),
/// and verbosity modes. It acts as the primary interface for logging
#[derive(Clone)]
pub struct Logger {
    pub gutter: String,
    pub verbosity: LoggingVerbosity,
}

impl Logger {
    /// Creates a logger with custom verbosity (e.g., for -q Minimal or Silent)
    pub fn with_verbosity(verbosity: LoggingVerbosity) -> Self {
        Self {
            verbosity,
            gutter: String::new(),
        }
    }

    /// Creates a new standard logger with empty indentation and full output
    pub fn new() -> Self {
        Self::with_verbosity(LoggingVerbosity::Detailed)
    }

    /// Creates a logger with hidden output (silent verbosity)
    pub fn silent() -> Self {
        Self::with_verbosity(LoggingVerbosity::Silent)
    }

    /// Creates a logger with minimal output (minimal verbosity)
    pub fn minimal() -> Self {
        Self::with_verbosity(LoggingVerbosity::Minimal)
    }

    /// Wraps Logger into Activity.
    /// Allows using methods like .action() inside functions accepting &mut Task as a parameter
    pub fn as_task(&self) -> Activity {
        Activity {
            log: self.clone(),
            message: String::new(),
            start: Instant::now(),
            is_last: true,
            parent_loggger: self.clone(),
            finished: true,
        }
    }

    /// Starts a new tracked task with its own lifecycle and duration
    pub fn step(&self, message: &str, is_last: bool) -> Activity {
        Activity::new(message.to_string(), is_last, self)
    }

    /// Starts a new tracked task with a custom icon and message
    pub fn step_with_icon<D: std::fmt::Display>(
        &self,
        icon: D,
        message: &str,
        is_last: bool,
    ) -> Activity {
        Activity::new_with_icon(message.to_string(), icon, is_last, self)
    }

    /// Logs a dim informational message intended to be used within a task
    pub fn info(&self, msg: &str) {
        if self.verbosity == LoggingVerbosity::Detailed {
            println!("{}{} {}", self.gutter, "•".dimmed(), msg.dimmed());
        }
    }

    /// Logs a warning message highlighted in yellow
    pub fn warn(&self, message: &str) {
        if self.verbosity != LoggingVerbosity::Silent {
            println!(
                "{} {} {}",
                self.gutter,
                "!".yellow().bold(),
                message.yellow()
            );
        }
    }

    /// Logs a final success message with a branch-like prefix
    pub fn success(&self, message: &str) {
        if self.verbosity == LoggingVerbosity::Detailed {
            let prefix = if self.gutter.is_empty() {
                "".to_string()
            } else {
                format!("{}", "└─ ".blue())
            };

            println!(
                "{}{}{} {}",
                self.gutter,
                prefix,
                "✓".green().bold(),
                message.green().bold()
            );
        }
    }

    /// Executes a closure as an atomic action, logging its completion and duration in one line.
    /// Perfect for fast operations that don't need detailed sub-steps
    pub fn action<F, R>(&self, message: &str, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let start: Instant = Instant::now();

        if self.verbosity == LoggingVerbosity::Detailed {
            print!("{}{} {}", self.gutter, "✓".green(), message.green().bold());
            let _ = io::stdout().flush();
        }

        let result: R = f();
        let duration = utils::format_duration(start.elapsed()).dimmed();

        if self.verbosity == LoggingVerbosity::Detailed {
            println!(" {}", duration);
        }

        result
    }

    /// Helper to check if the logger is in full detailed logging mode
    pub fn is_detailed(&self) -> bool {
        self.verbosity == LoggingVerbosity::Detailed
    }
}
