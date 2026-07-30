use super::traits::*;
use crate::{
    core::IrisEngine,
    infra::IrisPaths,
    models::Theme,
    modules::{Strategy, strategy::PipelineStep},
};

/// Main trait for all generators.
/// Acts as a purely declarative manifest describing application paths, configuration files,
/// and metadata, while delegating all execution and lifecycle mechanics to IrisEngine.
pub trait Generator: PathResolvable + Cleanable + Diagnosable {
    /// Returns the active strategy for applying themes
    fn strategy(&self) -> Strategy;

    /// Returns the list of steps for the pipeline (if there is such strategy specified)
    fn pipeline_steps(&self, _paths: &IrisPaths, _theme: &Theme) -> Vec<PipelineStep> {
        vec![]
    }

    /// Hook for specific action right before applying theme (config injection)
    fn pre_apply(&self, _engine: &IrisEngine) -> anyhow::Result<()> {
        Ok(())
    }

    /// Optional hook to inject custom data into the rendering pipeline (like bat syntax rules)
    fn enrich_context(&self, _context: &mut tera::Context, _theme: &Theme) -> anyhow::Result<()> {
        Ok(())
    }
}

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
pub enum GeneratorFilter {
    /// Enabled and installed
    Active,
    /// Installed but disabled
    Ready,
    /// Enabled but program not found in system
    Broken,
    /// Disabled and not found
    Missing,
}

impl GeneratorFilter {
    pub fn matches(&self, is_enabled: bool, is_installed: bool) -> bool {
        match self {
            Self::Active => is_enabled && is_installed,
            Self::Ready => !is_enabled && is_installed,
            Self::Broken => is_enabled && !is_installed,
            Self::Missing => !is_enabled && !is_installed,
        }
    }
}

/// Generator mock for registry test
#[cfg(test)]
pub struct GeneratorMock {
    pub name: &'static str,
    pub g_type: GeneratorType,
    pub installed: bool,
    pub strategy: Strategy,
}

#[cfg(test)]
impl Identifiable for GeneratorMock {
    fn name(&self) -> &str {
        self.name
    }

    fn generator_type(&self) -> GeneratorType {
        self.g_type
    }

    fn is_installed(&self) -> bool {
        self.installed
    }
}

#[cfg(test)]
impl PathResolvable for GeneratorMock {
    fn target_file_name(&self, _theme: &str) -> String {
        format!("{}.conf", self.name)
    }
}

#[cfg(test)]
impl Cleanable for GeneratorMock {
    fn cleanup(&self, _paths: &IrisPaths) -> anyhow::Result<()> {
        Ok(())
    }

    fn remove_theme(&self, _paths: &IrisPaths, _theme_name: &str) -> anyhow::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
impl Diagnosable for GeneratorMock {
    fn health_check(&self, _paths: &IrisPaths, _theme: &str) -> crate::models::HealthStatus {
        crate::models::HealthStatus::Ok
    }
}

#[cfg(test)]
impl Generator for GeneratorMock {
    fn strategy(&self) -> Strategy {
        self.strategy.clone()
    }
}
