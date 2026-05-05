use crate::{log::Reporter, utils};
use colored::Colorize;
use std::time::Instant;

/// Represents a specific unit of work with a start time and automatic cleanup
pub struct Task {
    pub log: Reporter,

    pub message: String,
    pub start: Instant,
    pub is_last: bool,
    pub parent_reporter: Reporter,
    pub finished: bool,
}

impl Task {
    /// Internal constructor for creating a task via `Reporter::step`
    pub fn new(message: String, is_last: bool, parent: &Reporter) -> Self {
        Self::new_with_icon(message, "●".blue().bold(), is_last, parent)
    }

    /// New constructor with custom icon support
    pub fn new_with_icon<D: std::fmt::Display>(
        message: String,
        icon: D,
        is_last: bool,
        parent: &Reporter,
    ) -> Self {
        if !parent.quiet {
            let icon_str: String = icon.to_string();
            let extra_space: &str = if icon_str.contains("●") { "" } else { " " };

            println!(
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
            parent_reporter: parent.clone(),
            log: Reporter {
                gutter: child_gutter,
                quiet: parent.quiet,
            },
            finished: false,
        }
    }

    /// Returns a muted version of the task to suppress internal logs during execution
    pub fn as_quiet(&self) -> Self {
        Self {
            message: self.message.clone(),
            start: self.start,
            is_last: self.is_last,
            parent_reporter: self.parent_reporter.clone(),
            log: Reporter {
                quiet: true,
                gutter: String::new(),
            },
            finished: false,
        }
    }

    /// Finalizes the task using the original message
    pub fn done(self) {
        let msg = self.message.clone();
        self.finish(&msg);
    }

    /// Finalizes the task with a custom result message
    pub fn done_with(self, msg: &str) {
        self.finish(msg);
    }

    /// Helper function to finish task
    fn finish(mut self, final_msg: &str) {
        if self.finished {
            return;
        }

        let duration = utils::format_duration(self.start.elapsed()).dimmed();

        if !self.parent_reporter.quiet {
            println!(
                "{}{} {} {}",
                self.parent_reporter.gutter,
                "✓".green(),
                final_msg.green().bold(),
                duration
            );

            if !self.is_last {
                println!("{}{}", self.parent_reporter.gutter, "│".dimmed());
            } else if self.parent_reporter.gutter.is_empty() {
                println!();
            }
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
    /// Use this for intermediate successful milestones within a task
    pub fn success(&self, msg: &str) {
        self.log.success(msg);
    }
}

impl Drop for Task {
    /// Ensures that if a task is dropped (e.g., due to a panic or error),
    /// it still prints an "incomplete" status to maintain log integrity
    fn drop(&mut self) {
        if !self.finished && !self.parent_reporter.quiet {
            println!("{}{}", self.parent_reporter.gutter, self.message.dimmed());
        }
    }
}
