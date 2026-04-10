pub mod multiplexer;
pub mod prompts;
pub mod registry;
pub mod system;
pub mod terminals;
pub mod tools;
pub mod traits;

pub use registry::GeneratorRegistry;
pub use traits::Generator;

use clap::ValueEnum;
use colored::Color;

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
