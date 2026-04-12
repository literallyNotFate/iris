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
        if self.finished {
            return;
        }

        if !self.quiet {
            print!("\r\x1B[K{}", self.build_done_output(is_last));
        } else {
            let duration: Duration = self.start_time.elapsed();
            let duration_str: String = self.format_duration(duration);
            println!("{}{}", "done".green(), duration_str);
        }

        let _ = io::stdout().flush();
        self.finished = true;
    }

    /// Logs an info sub-step (Level 2)
    pub fn info(&self, message: &str) {
        if !self.quiet {
            print!("{}", self.build_info_output(message));
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
        if !self.finished && !self.quiet {
            print!("{}", self.build_drop_output());
        }
    }
}

/// Build API (for testing)
impl Task {
    /// Build string for task done output
    fn build_done_output(&self, is_last: bool) -> String {
        let duration_str = self.format_duration(self.start_time.elapsed());
        let indent = "  ".repeat(self.level as usize);

        let branch = if self.level == 0 {
            "❯".blue().bold()
        } else if is_last {
            "└─".blue()
        } else {
            "├─".blue()
        };

        format!(
            "{}{} {} {} {}{}\n",
            indent,
            branch,
            self.message.dimmed(),
            ".".repeat(3).dimmed(),
            "done".green(),
            duration_str
        )
    }

    /// Build string for task info output
    fn build_info_output(&self, message: &str) -> String {
        let indent = "  ".repeat(self.level as usize);
        format!("{}  └─  {} {}\n", indent, "•", message)
            .dimmed()
            .to_string()
    }

    /// Build string for task drop output
    fn build_drop_output(&self) -> String {
        let label = if std::thread::panicking() {
            "failed".red().bold()
        } else {
            "skipped".dimmed()
        };
        format!("  {} {} {}\n", "⚠".yellow(), self.message.dimmed(), label)
    }
}

/// Unit-tests for task logger
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn should_handle_task_done_branches() {
        let task: Task = Task::new("Step".to_string(), 1, false);

        let output_mid: String = task.build_done_output(false);
        assert!(output_mid.contains("├─"));

        let output_last: String = task.build_done_output(true);
        assert!(output_last.contains("└─"));
    }

    #[test]
    fn should_format_duration_correctly() {
        let task: Task = Task::new("Time".to_string(), 0, false);
        assert_eq!(task.format_duration(Duration::from_millis(50)), "");

        let ms_output: String = task.format_duration(Duration::from_millis(250));
        assert!(ms_output.contains("250ms"));

        let s_output: String = task.format_duration(Duration::from_secs(2));
        assert!(s_output.contains("2.00s"));
    }

    #[test]
    fn should_handle_task_info_output() {
        let task: Task = Task::new("Parent".to_string(), 1, false);
        let output: String = task.build_info_output("Child info");

        assert!(output.contains("  "));
        assert!(output.contains("•"));
        assert!(output.contains("Child info"));
    }

    #[test]
    fn should_handle_task_drop_output() {
        let task: Task = Task::new("Abandoned".to_string(), 0, false);
        let output: String = task.build_drop_output();

        assert!(output.contains("⚠"));
        assert!(output.contains("skipped"));
    }

    #[test]
    fn should_respect_finished_flag_in_done() {
        let mut task: Task = Task::new("Finished".to_string(), 0, false);
        task.done(true);
        assert!(task.finished);

        task.done(true);
    }
}
