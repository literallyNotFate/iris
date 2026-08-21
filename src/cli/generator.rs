use crate::modules::{GeneratorFilter, GeneratorType};

#[derive(clap::Subcommand)]
pub enum GenAction {
    /// Enable a specific generator or all discovered ones
    Enable {
        #[arg(value_name = "GENERATOR", required_unless_present = "all")]
        name: Option<String>,

        /// Enable all discovered supported apps
        #[arg(long, conflicts_with = "name")]
        all: bool,
    },

    /// Disable a specific generator or all active ones
    Disable {
        #[arg(value_name = "GENERATOR", required_unless_present = "all")]
        name: Option<String>,

        /// Disable all active generators
        #[arg(long, conflicts_with = "name")]
        all: bool,
    },

    /// Open an interactive TUI selector to manage generators
    Select,

    /// List available generators and their current status
    List {
        /// Filter generators by type (e.g., 'tool', 'terminal')
        #[arg(short = 't', long = "type", value_name = "TYPE")]
        generator_type: Option<GeneratorType>,

        /// Filter generators by their operational state
        #[arg(short = 's', long = "status", value_name = "STATE")]
        status: Option<GeneratorFilter>,
    },
}
