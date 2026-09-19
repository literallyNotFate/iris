use crate::{
    infra::IrisPaths,
    models::{HealthStatus, Issue},
};
use std::{
    env, fs,
    path::{Path, PathBuf},
};

/// Check rules
#[derive(Debug, Clone)]
pub enum CheckRule {
    /// Check whether a configuration file exists
    ConfigExists,
    /// Check for the presence of theme injection markers/tags in the file
    MarkersExist,
    /// Check the validity of the symlink and cache consistency (if a theme is passed)
    SymlinkValid,
    /// Call custom content validation (`validate_content`)
    ContentValid,
    /// Check if the generated file exists in the cache
    CacheExists,
    /// Check an environment variable (var_name, function to compute expected path from paths)
    EnvValid(&'static str, fn(&IrisPaths) -> PathBuf),
}

/// Handles environment health diagnostics, validation, and automated fixes for generators
pub trait Diagnosable: super::PathResolvable {
    /// List of rules for a specific generator (empty by default)
    fn check_rules(&self) -> &[CheckRule] {
        &[]
    }

    /// Custom content validation (can be overridden if necessary)
    fn validate_content(&self, _content: &str, _theme: &str) -> Option<HealthStatus> {
        None
    }

    /// Performs a comprehensive health check on the generator's environment,
    /// configuration files, and symlinks
    fn check(&self, paths: &IrisPaths, theme: &str) -> HealthStatus {
        if !self.is_installed() {
            return HealthStatus::error(Issue::BinaryNotFound);
        }

        self.check_rules()
            .iter()
            .find_map(|rule| {
                let status = match rule {
                    CheckRule::ConfigExists => {
                        HealthStatus::check_file(&self.config_path(paths), Issue::ConfigMissing)
                    }
                    CheckRule::MarkersExist => self.check_markers(paths, theme),
                    CheckRule::SymlinkValid => self.check_symlink_target(paths, theme),
                    CheckRule::ContentValid => self.check_content(paths, theme),
                    CheckRule::CacheExists => self.check_cache(paths, theme),
                    CheckRule::EnvValid(var_name, path_fn) => {
                        let expected_path: PathBuf = path_fn(paths);
                        self.check_env(var_name, &expected_path)
                    }
                };

                if status.is_ok() { None } else { Some(status) }
            })
            .unwrap_or(HealthStatus::Ok)
    }

    /// Helper method to check markers
    fn check_markers(&self, paths: &IrisPaths, theme: &str) -> HealthStatus {
        let content: String = fs::read_to_string(self.link_path(paths, theme)).unwrap_or_default();
        let (start, end): (String, String) = (
            format!("# [iris:begin:{}]", self.name()),
            format!("# [iris:end:{}]", self.name()),
        );
        if content.contains(&start) && content.contains(&end) {
            HealthStatus::Ok
        } else {
            HealthStatus::warn(Issue::MarkerMissing)
        }
    }

    /// Helper method to check symlink
    fn check_symlink_target(&self, paths: &IrisPaths, theme: &str) -> HealthStatus {
        let link_path: PathBuf = self.link_path(paths, theme);
        let status = HealthStatus::check_symlink(&link_path, Issue::SymlinkInvalid);
        if !status.is_ok() || theme.is_empty() {
            return status;
        }

        let expected = self.cache_path(paths, theme);
        fs::read_link(&link_path)
            .map(|target| {
                let resolved: PathBuf = if target.is_relative() {
                    link_path
                        .parent()
                        .map(|p| p.join(&target))
                        .unwrap_or(target)
                } else {
                    target
                };
                if resolved != expected {
                    HealthStatus::warn(Issue::CacheMismatch)
                } else {
                    HealthStatus::Ok
                }
            })
            .unwrap_or(HealthStatus::Ok)
    }

    /// Helper method to check content
    fn check_content(&self, paths: &IrisPaths, theme: &str) -> HealthStatus {
        if theme.is_empty() {
            return HealthStatus::Ok;
        }

        let config_path: PathBuf = self.config_path(paths);
        let content: String = fs::read_to_string(&config_path).unwrap_or_default();
        self.validate_content(&content, theme)
            .unwrap_or(HealthStatus::Ok)
    }

    /// Helper method to check cache
    fn check_cache(&self, paths: &IrisPaths, theme: &str) -> HealthStatus {
        if !theme.is_empty() && !self.cache_path(paths, theme).exists() {
            HealthStatus::warn(Issue::CacheMissing)
        } else {
            HealthStatus::Ok
        }
    }

    /// Helper method to check env
    fn check_env(&self, var_name: &str, expected: &Path) -> HealthStatus {
        let current: String = env::var(var_name).unwrap_or_default();
        if current != expected.to_string_lossy() {
            HealthStatus::warn(Issue::EnvMismatch)
        } else {
            HealthStatus::Ok
        }
    }
}
