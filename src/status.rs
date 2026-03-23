use colored::*;

/// Struct to show status of current operation
pub struct Status;

impl Status {
    // Helper function to create levels of indent for nesting
    fn get_indent(level: u8) -> String {
        "  ".repeat(level as usize)
    }

    pub fn step(message: &str, level: u8) {
        let p = Self::get_indent(level);
        println!("{} {} {}", p, "➜".blue().bold(), message.trim_start());
    }

    pub fn success(message: &str, level: u8) {
        let p = Self::get_indent(level);
        println!("{} {} {}", p, "✔".green().bold(), message.trim_start());
    }

    pub fn error(message: &str, level: u8) {
        let p = Self::get_indent(level);
        println!("{} {} {}", p, "✘".red().bold(), message.trim_start());
    }
}
