use crate::{log::Task, utils};
use colored::*;
use std::{
    io::{self, Write},
    time::Instant,
};

/// A reporter responsible for handling the output format, indentation (gutter),
/// and silence modes. It acts as the primary interface for logging
#[derive(Clone)]
pub struct Reporter {
    pub gutter: String,
    pub quiet: bool,
}

impl Reporter {
    /// Creates a new standard reporter with empty indentation
    pub fn new() -> Self {
        Self {
            quiet: false,
            gutter: String::new(),
        }
    }

    /// Creates a reporter that suppresses all output
    pub fn quiet() -> Self {
        Self {
            quiet: true,
            gutter: String::new(),
        }
    }

    /// Wraps Reporter into Task.
    /// Allows using methods like .action() inside functions accepting &mut Task as a parameter
    pub fn as_task(&self) -> Task {
        Task {
            log: self.clone(),
            message: String::new(),
            start: Instant::now(),
            is_last: true,
            parent_reporter: self.clone(),
            finished: true,
        }
    }

    /// Starts a new tracked task with its own lifecycle and duration
    pub fn step(&self, message: &str, is_last: bool) -> Task {
        Task::new(message.to_string(), is_last, self)
    }

    /// Starts a new tracked task with a custom icon and message
    pub fn step_with_icon<D: std::fmt::Display>(
        &self,
        icon: D,
        message: &str,
        is_last: bool,
    ) -> Task {
        Task::new_with_icon(message.to_string(), icon, is_last, self)
    }

    /// Logs a dim informational message intended to be used within a task
    pub fn info(&self, msg: &str) {
        if self.quiet {
            return;
        }

        println!("{}{} {}", self.gutter, "•".dimmed(), msg.dimmed());
    }

    /// Logs a warning message highlighted in yellow
    pub fn warn(&self, message: &str) {
        if self.quiet {
            return;
        }

        println!(
            "{} {} {}",
            self.gutter,
            "!".yellow().bold(),
            message.yellow()
        );
    }

    /// Logs a final success message with a branch-like prefix
    pub fn success(&self, message: &str) {
        if self.quiet {
            return;
        }

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

    /// Executes a closure as an atomic action, logging its completion and duration in one line.
    /// Perfect for fast operations that don't need detailed sub-steps
    pub fn action<F, R>(&self, message: &str, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let start: Instant = Instant::now();

        if !self.quiet {
            print!("{}{} {}", self.gutter, "✓".green(), message.green().bold());
            let _ = io::stdout().flush();
        }

        let result: R = f();
        let duration = utils::format_duration(start.elapsed()).dimmed();

        if !self.quiet {
            print!(" {}", duration);
        }

        result
    }
}
