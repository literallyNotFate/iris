use crate::{core::IrisContext, models::Palette};
use clap::ValueEnum;
use colored::Color;
use std::{env, path::PathBuf};

/// Main trait for all generators
pub trait Generator: Send + Sync {
    /// Returns name of the generator (e.g "ghostty")
    fn name(&self) -> &str;

    /// Returns type of the generator
    fn generator_type(&self) -> GeneratorType;

    /// Returns the name of the file responsible for configuring app
    fn target_file_name(&self, theme: &str) -> String;

    /// Checks whether this tool is installed
    fn is_installed(&self) -> bool {
        which::which(self.name()).is_ok()
    }

    /// Logic of applying the theme (file writing, building cache etc)
    fn apply(&self, p: &Palette, ctx: &IrisContext) -> anyhow::Result<()>;

    /// Automatic config directory resolver
    fn resolve_config_directory(&self) -> PathBuf {
        if let Some(p) = self.env_config_directory() {
            if p.is_file() {
                return p.parent().unwrap_or(&p).to_path_buf();
            }

            return p;
        }

        if let Some(home) = dirs::home_dir() {
            let config_path = home.join(".config").join(self.name());
            return config_path;
        }

        let home = env::var("HOME").unwrap_or_else(|_| ".".into());
        PathBuf::from(home).join(".config").join(self.name())
    }

    /// Allows module to pass environment value path of the config (e.g STARSHIP_CONFIG)
    fn env_config_directory(&self) -> Option<PathBuf> {
        None
    }

    /// Optional post-apply hint (e.g. "add import to config")
    fn setup_hint(&self) -> Option<String> {
        None
    }
}

/// Generator type for specific module
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum GeneratorType {
    Terminal,
    Tool,
    Prompt,
    Multiplexer,
    System,
}

impl GeneratorType {
    /// Returns the icon based on generator type
    pub fn icon(&self) -> &str {
        match self {
            Self::Terminal => "󰞷",
            Self::Tool => "󰆍",
            Self::Prompt => "󱆃",
            Self::Multiplexer => "󱂬",
            Self::System => "󰢮",
        }
    }

    /// Returns the color based on generator type
    pub fn color(&self) -> Color {
        match self {
            Self::Terminal => Color::Blue,
            Self::Tool => Color::Magenta,
            Self::Prompt => Color::Cyan,
            Self::Multiplexer => Color::Green,
            Self::System => Color::Yellow,
        }
    }

    /// Returns the label based on generator type
    pub fn label(&self) -> &str {
        match self {
            Self::Terminal => "term",
            Self::Tool => "cli",
            Self::Prompt => "prompt",
            Self::Multiplexer => "mux",
            Self::System => "sys",
        }
    }
}
