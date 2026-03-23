use colored::*;
use std::time::Instant;

/// Struct to show status of current operation
pub struct Status;

/// Status task with timer
pub struct Task {
    message: String,
    start_time: Instant,
    level: u8,
}

impl Status {
    // Helper function to create levels of indent for nesting
    fn get_indent(level: u8) -> String {
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

    /// Error without timer
    pub fn error(message: &str, level: u8) {
        let p = Self::get_indent(level);
        println!("{} {} {}", p, "✘".red().bold(), message.trim_start());
    }
}

impl Task {
    /// Finalizing task with elapsed time
    pub fn done(self, custom_message: Option<&str>) {
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
    pub fn fail(self, error: &str) {
        let indent = Status::get_indent(self.level);
        println!(
            "{} {} {} {}",
            indent,
            "✘".red().bold(),
            self.message,
            format!("(failed: {})", error).red()
        );
    }
}
