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
    fn data(&self) -> (&str, Color, &str) {
        match self {
            Self::Terminal => ("󰞷", Color::Blue, "terminals"),
            Self::Tool => ("󰆍", Color::Magenta, "tools"),
            Self::Prompt => ("󱆃", Color::Cyan, "prompts"),
            Self::Multiplexer => ("󱂬", Color::Green, "multiplexer"),
            Self::System => ("󰢮", Color::Yellow, "system"),
        }
    }

    pub fn icon(&self) -> &str {
        self.data().0
    }
    pub fn color(&self) -> Color {
        self.data().1
    }
    pub fn label(&self) -> &str {
        self.data().2
    }
}
