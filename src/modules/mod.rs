pub mod multiplexer;
pub mod prompts;
pub mod registry;
pub mod system;
pub mod terminals;
pub mod tools;
pub mod traits;

pub use registry::GeneratorRegistry;
pub use traits::Generator;

/// Generator type for specific module
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum)]
pub enum GeneratorType {
    Terminal,
    Tool,
    Prompt,
    Multiplexer,
    System,
}

impl GeneratorType {
    fn data(&self) -> (&str, colored::Color, &str) {
        match self {
            Self::Terminal => ("󰞷", colored::Color::Blue, "terminals"),
            Self::Tool => ("󰆍", colored::Color::Magenta, "tools"),
            Self::Prompt => ("󱆃", colored::Color::Cyan, "prompts"),
            Self::Multiplexer => ("󱂬", colored::Color::Green, "multiplexer"),
            Self::System => ("󰢮", colored::Color::Yellow, "system"),
        }
    }

    pub fn icon(&self) -> &str {
        self.data().0
    }
    pub fn color(&self) -> colored::Color {
        self.data().1
    }
    pub fn label(&self) -> &str {
        self.data().2
    }
}

/// Generator filter
#[derive(clap::ValueEnum, Clone, Copy, PartialEq, Eq)]
pub enum StateFilter {
    /// Enabled and installed
    Active,
    /// Installed but disabled
    Ready,
    /// Enabled but program not found in system
    Broken,
    /// Disabled and not found
    Missing,
}

impl StateFilter {
    pub fn matches(&self, is_enabled: bool, is_installed: bool) -> bool {
        match self {
            Self::Active => is_enabled && is_installed,
            Self::Ready => !is_enabled && is_installed,
            Self::Broken => is_enabled && !is_installed,
            Self::Missing => !is_enabled && !is_installed,
        }
    }
}
