use crate::{
    core::IrisEngine,
    infra::IrisPaths,
    log::Activity,
    models::{HealthStatus, Theme},
    modules::{Cleanable, Strategy, strategy::PipelineStep},
};

/// Main trait for all generators.
/// Acts as a purely declarative manifest describing application paths, configuration files,
/// and metadata, while delegating all execution and lifecycle mechanics to IrisEngine.
pub trait Generator: Send + Sync {
    /// Returns name of the generator (e.g "ghostty")
    fn name(&self) -> &str;

    /// Returns type of the generator (e.g Terminal, Tool)
    fn generator_type(&self) -> GeneratorType;

    /// Returns the active strategy for applying themes
    fn strategy(&self) -> Strategy;

    /// Returns the list of steps for the pipeline (if there is such strategy specified)
    fn pipeline_steps(&self, _paths: &IrisPaths, _theme: &Theme) -> Vec<PipelineStep> {
        vec![]
    }

    /// Returns the name of the file responsible for configuring the app
    fn target_file_name(&self, theme: &str) -> String;

    /// Generator path in cache.
    /// By default: ~/.cache/iris/gen/[name]/[target_file]
    fn cache_path(&self, paths: &IrisPaths, theme: &str) -> std::path::PathBuf {
        paths
            .generators
            .join(self.name())
            .join(self.target_file_name(theme))
    }

    /// Path where the application expects to find its config or active theme file.
    /// By default: ~/.config/[name]/[target_file]
    fn link_path(&self, paths: &IrisPaths, theme: &str) -> std::path::PathBuf {
        self.resolve_config_directory(paths)
            .join(self.target_file_name(theme))
    }

    /// Returns path to the static active link if the app uses it.
    /// By default: None (means that app uses dynamic theme imports like btop etc.)
    fn active_link_path(&self, _paths: &IrisPaths) -> Option<std::path::PathBuf> {
        None
    }

    /// Dynamically forms template identification path for Tera.
    /// E.g. "tool/yazi", "terminal/alacritty"
    fn template_path(&self) -> String {
        format!("{}/{}", self.generator_type().label(), self.name())
    }

    /// Path to the current active theme file
    fn theme_path(&self, paths: &IrisPaths, theme: &str) -> std::path::PathBuf {
        self.link_path(paths, theme)
    }

    /// Checks whether this specific tool binary is installed on the system
    fn is_installed(&self) -> bool {
        which::which(self.name()).is_ok()
    }

    /// Automatic config base directory resolver
    fn resolve_config_directory(&self, paths: &IrisPaths) -> std::path::PathBuf {
        if let Some(p) = self.env_config_directory() {
            return if p.is_file() {
                p.parent().unwrap_or(&p).to_path_buf()
            } else {
                p
            };
        }

        let config_base = paths
            .config
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| paths.config.clone());

        config_base.join(self.name())
    }

    /// Allows custom modules to pass environment value path overrides (e.g. STARSHIP_CONFIG)
    fn env_config_directory(&self) -> Option<std::path::PathBuf> {
        None
    }

    /// "Health" status check.
    /// Implementation by default falls back to a clean state.
    fn health_check(&self, _paths: &IrisPaths, _theme: &str) -> HealthStatus {
        HealthStatus::Ok
    }

    /// Automatically fix detected environment issues (based on HealthStatus)
    fn fix(
        &self,
        status: &HealthStatus,
        engine: &IrisEngine,
        activity: &mut Activity,
    ) -> anyhow::Result<()> {
        match status {
            HealthStatus::Ok => Ok(()),
            HealthStatus::Issue(_severity, issue, _hint) => {
                let msg: String = format!("Repaired `{}` issue: {}", self.name(), issue);
                activity
                    .log
                    .action(&msg, || engine.execute_apply(self, &mut activity.muted()))
            }
        }
    }

    /// Hook for specific action right before applying theme (config injection)
    fn pre_apply(&self, _engine: &IrisEngine) -> anyhow::Result<()> {
        Ok(())
    }

    /// Optional hook to inject custom data into the rendering pipeline (like bat syntax rules)
    fn enrich_context(&self, _context: &mut tera::Context, _theme: &Theme) -> anyhow::Result<()> {
        Ok(())
    }

    /// Optional method to obtain interface for cleaning/remove theme
    fn as_cleanable(&self) -> Option<&dyn Cleanable> {
        None
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
impl Generator for GeneratorMock {
    fn name(&self) -> &str {
        self.name
    }

    fn generator_type(&self) -> GeneratorType {
        self.g_type
    }

    fn strategy(&self) -> Strategy {
        self.strategy.clone()
    }

    fn target_file_name(&self, theme: &str) -> String {
        format!("{}.conf", theme)
    }

    fn is_installed(&self) -> bool {
        self.installed
    }

    fn fix(
        &self,
        status: &HealthStatus,
        engine: &IrisEngine,
        activity: &mut Activity,
    ) -> anyhow::Result<()> {
        if !status.is_ok() {
            return engine.execute_apply(self, activity);
        }
        Ok(())
    }

    fn enrich_context(&self, _: &mut tera::Context, _: &Theme) -> anyhow::Result<()> {
        Ok(())
    }
}
