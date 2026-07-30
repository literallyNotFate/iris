use crate::infra::IrisPaths;
use std::path::PathBuf;

/// Handles file system paths, target configurations, and installation checks
pub trait PathResolvable: super::Identifiable {
    /// Returns the name of the configuration file produced for a specific theme
    fn target_file_name(&self, theme: &str) -> String;

    /// Returns the full path to the cached template output file.
    /// Default: `~/.cache/iris/gen/[name]/[target_file]`
    fn cache_path(&self, paths: &IrisPaths, theme: &str) -> PathBuf {
        paths
            .generators
            .join(self.name())
            .join(self.target_file_name(theme))
    }

    /// Returns the path where the application expects its active configuration or theme file.
    /// Default: `~/.config/[name]/[target_file]`
    fn link_path(&self, paths: &IrisPaths, theme: &str) -> PathBuf {
        self.resolve_config_directory(paths)
            .join(self.target_file_name(theme))
    }

    /// Returns the path to a static active symlink, if used by the application.
    /// Default: `None` (meaning the application uses dynamic theme imports)
    fn active_link_path(&self, _paths: &IrisPaths) -> Option<PathBuf> {
        None
    }

    /// Resolves the base configuration directory for the application
    fn resolve_config_directory(&self, paths: &IrisPaths) -> PathBuf {
        if let Some(p) = self.env_config_directory() {
            return if p.is_file() {
                p.parent().unwrap_or(&p).to_path_buf()
            } else {
                p
            };
        }

        let config_base: PathBuf = paths
            .config
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| paths.config.clone());

        config_base.join(self.name())
    }

    /// Optional environment override for the configuration directory (e.g., `STARSHIP_CONFIG`)
    fn env_config_directory(&self) -> Option<PathBuf> {
        None
    }

    /// Returns the path to the current active theme file
    fn theme_path(&self, paths: &IrisPaths, theme: &str) -> PathBuf {
        self.link_path(paths, theme)
    }

    /// Dynamically forms template identification path for Tera.
    /// E.g. "tool/yazi", "terminal/alacritty"
    fn template_path(&self) -> String {
        format!("{}/{}", self.generator_type().label(), self.name())
    }
}
