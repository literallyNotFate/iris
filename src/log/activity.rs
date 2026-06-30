use crate::{
    log::{Logger, LoggingVerbosity},
    utils,
};
use colored::Colorize;
use std::time::Instant;

/// Represents a specific unit of work with a start time and automatic cleanup
pub struct Activity {
    pub log: Logger,

    pub message: String,
    pub start: Instant,
    pub is_last: bool,
    pub parent_logger: Logger,
    pub finished: bool,
}

impl Activity {
    /// Internal constructor for creating a task via `Reporter::step`
    pub fn new(message: String, is_last: bool, parent: &Logger) -> Self {
        Self::new_with_icon(message, "●".blue().bold(), is_last, parent)
    }

    /// New constructor with custom icon support
    pub fn new_with_icon<D: std::fmt::Display>(
        message: String,
        icon: D,
        is_last: bool,
        parent: &Logger,
    ) -> Self {
        if parent.verbosity == LoggingVerbosity::Detailed {
            let icon_str = icon.to_string();
            let extra_space = if icon_str.contains("●") { "" } else { " " };

            eprintln!(
                "{}{} {}{}",
                parent.gutter,
                icon,
                extra_space,
                message.white().bold()
            );
        }

        let child_gutter: String = format!("{}{}", parent.gutter, "│  ".dimmed());

        Self {
            message,
            start: Instant::now(),
            is_last,
            parent_logger: parent.clone(),
            log: Logger {
                gutter: child_gutter,
                verbosity: parent.verbosity,
            },
            finished: false,
        }
    }

    /// Returns a muted version of the activity to suppress internal logs during execution
    pub fn muted(&self) -> Self {
        Self {
            message: self.message.clone(),
            start: self.start,
            is_last: self.is_last,
            parent_logger: self.parent_logger.clone(),
            log: Logger {
                verbosity: LoggingVerbosity::Silent,
                gutter: String::new(),
            },
            finished: false,
        }
    }

    /// Finalizes the activity using the original message
    pub fn done(self) {
        let msg = self.message.clone();
        self.finish(&msg);
    }

    /// Finalizes the activity with a custom result message
    pub fn done_with(self, msg: &str) {
        self.finish(msg);
    }

    /// Helper function to finish activity
    fn finish(mut self, final_msg: &str) {
        if self.finished {
            return;
        }

        let duration = utils::format_duration(self.start.elapsed()).dimmed();

        match self.parent_logger.verbosity {
            LoggingVerbosity::Detailed => {
                eprintln!(
                    "{}{} {} {}",
                    self.parent_logger.gutter,
                    "✓".green(),
                    final_msg.green().bold(),
                    duration
                );

                if !self.is_last {
                    eprintln!("{}{}", self.parent_logger.gutter, "│".dimmed());
                } else if self.parent_logger.gutter.is_empty() {
                    eprintln!();
                }
            }
            LoggingVerbosity::Minimal => {
                if self.parent_logger.gutter.is_empty() {
                    eprintln!(
                        "{} {} {}",
                        "✓".green().bold(),
                        final_msg.green().bold(),
                        duration
                    );
                }
            }
            LoggingVerbosity::Silent => {}
        }

        self.finished = true;
        std::mem::forget(self);
    }

    /// Logs a dim informational message
    pub fn info(&self, msg: &str) {
        self.log.info(msg);
    }

    /// Logs a warning message in yellow
    pub fn warn(&self, msg: &str) {
        self.log.warn(msg);
    }

    /// Logs a success message with a branch prefix.
    /// Use this for intermediate successful milestones within activity
    pub fn success(&self, msg: &str) {
        self.log.success(msg);
    }
}

impl Drop for Activity {
    /// Ensures that if a activity is dropped (e.g., due to a panic or error),
    /// it still prints an "incomplete" status to maintain log integrity
    fn drop(&mut self) {
        if !self.finished && self.parent_logger.verbosity == LoggingVerbosity::Detailed {
            eprintln!("{}{}", self.parent_logger.gutter, self.message.dimmed());
        }
    }
}
