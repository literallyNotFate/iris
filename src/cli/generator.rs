use crate::modules::{GeneratorFilter, GeneratorType};
use clap::Subcommand;

#[derive(Subcommand)]
pub enum GenAction {
    /// Automatically enable generators for all supported apps found in the system
    Auto,

    /// Enable a specific generator
    Enable {
        #[arg(value_name = "GENERATOR")]
        name: String,
    },

    /// Disable a specific generator
    Disable {
        #[arg(value_name = "GENERATOR")]
        name: String,
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
