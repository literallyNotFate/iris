use colored::*;
use std::{
    io::{self, Write},
    time::{Duration, Instant},
};

/// Represents an ongoing operation with a timer
pub struct Task {
    message: String,
    start_time: Instant,
    level: u8,
    quiet: bool,
    finished: bool,
}

impl Task {
    pub fn new(message: String, level: u8, quiet: bool) -> Self {
        Self {
            message,
            start_time: Instant::now(),
            level,
            quiet,
            finished: false,
        }
    }

    /// Finalizes the task and prints the result.
    ///
    /// In `quiet` mode, it appends "ready" or the time to the current line.
    /// In normal mode, it overwrites the line with a full branch (├─ or └─)
    pub fn done(&mut self, is_last: bool) {
        if self.finished || self.quiet {
            return;
        }

        let duration_str = self.format_duration(self.start_time.elapsed());
        let indent = "  ".repeat(self.level as usize);

        let branch = if self.level == 0 {
            "❯".blue().bold()
        } else if is_last {
            "└─".blue()
        } else {
            "├─".blue()
        };

        print!("\r\x1B[K");
        println!(
            "{}{} {} {} {}{}",
            indent,
            branch,
            self.message.dimmed(),
            ".".repeat(3).dimmed(),
            "done".green(),
            duration_str
        );

        let _ = io::stdout().flush();
        self.finished = true;
    }

    /// Logs an info sub-step (Level 2)
    pub fn info(&self, message: &str) {
        if !self.quiet {
            let indent = "  ".repeat(self.level as usize);
            let text = format!("{}  └─  {} {}", indent, "•", message).dimmed();

            println!("{}", text);
        }
    }

    /// Formats Duration into a human-readable string
    fn format_duration(&self, duration: Duration) -> String {
        let millis = duration.as_millis();
        if millis < 100 {
            return "".to_string();
        }

        let text = if duration.as_secs() >= 1 {
            format!("{:.2}s", duration.as_secs_f32())
        } else {
            format!("{}ms", millis)
        };

        format!(" {}{}{}", "[".dimmed(), text.yellow().bold(), "]".dimmed())
    }
}

/// Made for dealing with panic (interrupting)
impl Drop for Task {
    fn drop(&mut self) {
        if !self.finished {
            if self.quiet {
                return;
            }

            let label = if std::thread::panicking() {
                "failed".red().bold()
            } else {
                "skipped".dimmed()
            };

            println!("  {} {} {}", "⚠".yellow(), self.message.dimmed(), label);
        }
    }
}
