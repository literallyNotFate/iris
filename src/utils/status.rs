use colored::*;
use std::time::Instant;

/// Struct to show status of current operation
pub struct Status;

/// Status task with timer
pub struct Task {
    message: String,
    start_time: Instant,
    level: u8,
    finished: bool,
}

impl Status {
    // Helper function to create levels of indent for nesting
    pub fn get_indent(level: u8) -> String {
        "  ".repeat(level as usize)
    }

    /// Begins task
    pub fn step(message: &str, level: u8) -> Task {
        println!(
            "{} {} {}",
            Self::get_indent(level),
            "➜".blue().bold(),
            message
        );
        Task {
            message: message.to_string(),
            start_time: Instant::now(),
            level,
            finished: false,
        }
    }

    /// Success without timer (for simple tasks)
    pub fn success(message: &str, level: u8) {
        println!(
            "{} {} {}",
            Self::get_indent(level),
            "✔".green().bold(),
            message
        );
    }

    /// Warning without timer
    pub fn warn(message: &str, level: u8) {
        println!(
            "{} {} {}",
            Self::get_indent(level),
            "!".yellow().bold(),
            message.yellow()
        );
    }

    /// Error without timer
    pub fn error(message: &str, level: u8) {
        let p = Self::get_indent(level);
        println!("{} {} {}", p, "✘".red().bold(), message.trim_start());
    }
}

impl Task {
    /// Finalizing task with elapsed time
    pub fn done(mut self, custom_message: Option<&str>) {
        self.finished = true;
        let duration = self.start_time.elapsed();

        let time_str = if duration.as_millis() > 0 {
            let formatted_time = if duration.as_secs() > 0 {
                format!("{:.2}s", duration.as_secs_f32())
            } else {
                format!("{}ms", duration.as_millis())
            };

            format!(" {}", formatted_time.bright_yellow().bold())
        } else {
            String::new()
        };

        let msg = custom_message.unwrap_or(&self.message);
        let indent = Status::get_indent(self.level);

        println!("{} {} {}{}", indent, "✔".green().bold(), msg, time_str);
    }

    /// Task fail with elapsed time
    pub fn fail(mut self, error: &str) {
        self.finished = true;
        let indent = Status::get_indent(self.level);
        println!(
            "{} {} {} {}",
            indent,
            "✘".red().bold(),
            self.message,
            format!("(failed: {})", error).red()
        );
    }

    /// To show info message for task
    pub fn info(&self, message: &str) {
        let indent = Status::get_indent(self.level + 1);
        println!("{} {} {}", indent, "•".dimmed(), message.dimmed());
    }
}

/// Made for dealing with panic (interrupting)
impl Drop for Task {
    fn drop(&mut self) {
        if !self.finished && !std::thread::panicking() {
            let indent = Status::get_indent(self.level);
            println!(
                "{} {} {} {}",
                indent,
                "⚠".yellow().bold(),
                self.message,
                "(interrupted)".dimmed()
            );
        }
    }
}
