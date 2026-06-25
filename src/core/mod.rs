pub mod bridge;
pub mod context;
pub mod orchestrator;
pub mod paths;
pub mod setup;
pub mod templater;

pub use bridge::NeovimBridge;
pub use context::IrisContext;
pub use orchestrator::ThemeOrchestrator;
pub use paths::IrisPaths;
pub use setup::IrisSetup;
pub use templater::Templater;
