use crate::{infra::IrisPaths, models::HealthStatus};

/// Handles environment health diagnostics, validation, and automated fixes for generators
pub trait Diagnosable: super::PathResolvable {
    /// Performs a comprehensive health check on the generator's environment,
    /// configuration files, and symlinks
    fn health_check(&self, _paths: &IrisPaths, _theme: &str) -> HealthStatus {
        HealthStatus::Ok
    }
}
