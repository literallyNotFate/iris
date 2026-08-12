use crate::infra::IrisPaths;
use std::path::PathBuf;

/// Configuration source type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigSource {
    /// Standard path to the config directory (`~/.config/[name]/...`)
    Default,
    /// Must be a file specified via an environment variable or in the config
    File(PathBuf),
    /// Directory specified via an environment variable
    Dir(PathBuf),
}

/// Handles file system paths, target configurations, and installation checks
pub trait PathResolvable: super::Identifiable {
    /// Name of the application's main configuration file (e.g., "alacritty.toml", "tmux.conf")
    fn base_file_name(&self) -> String;

    /// Name of the generated theme file. By default, falls back to `base_file_name`.
    fn file_name(&self, _theme: &str) -> String {
        self.base_file_name()
    }

    /// Defines the configuration source for a specific generator
    fn config_source(&self) -> ConfigSource {
        ConfigSource::Default
    }

    /// Returns the base configuration directory for the application
    fn config_dir(&self, paths: &IrisPaths) -> PathBuf {
        match self.config_source() {
            ConfigSource::File(path) => path.parent().map(|p| p.to_path_buf()).unwrap_or(path),
            ConfigSource::Dir(path) => path,
            ConfigSource::Default => {
                let base = paths.config.parent().unwrap_or(&paths.config);
                base.join(self.name())
            }
        }
    }

    /// Path to the actual main configuration file that the application reads (e.g., config.toml, alacritty.toml)
    /// Default: `~/.config/[name]/[file_name]` (where file_name is evaluated with an empty theme)
    fn config_path(&self, paths: &IrisPaths) -> PathBuf {
        match self.config_source() {
            ConfigSource::File(path) => path,
            ConfigSource::Dir(path) => path.join(self.base_file_name()),
            ConfigSource::Default => self.config_dir(paths).join(self.base_file_name()),
        }
    }

    /// Returns the full path to the cached template output file.
    /// Default: `~/.cache/iris/gen/[name]/[file_name]`
    fn cache_path(&self, paths: &IrisPaths, theme: &str) -> PathBuf {
        paths
            .generators
            .join(self.name())
            .join(self.file_name(theme))
    }

    /// Returns the path where the application expects its theme file or link.
    /// Default: `~/.config/[name]/[file_name(theme)]`
    fn link_path(&self, paths: &IrisPaths, theme: &str) -> PathBuf {
        self.config_dir(paths).join(self.file_name(theme))
    }

    /// Returns the path to a static active symlink, if used by the application.
    /// Default: `None` (meaning the application uses dynamic theme imports)
    fn active_link_path(&self, _paths: &IrisPaths) -> Option<PathBuf> {
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
