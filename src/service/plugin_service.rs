use crate::{
    infra::{IrisPaths, NeovimBridge},
    log::Logger,
    models::PluginManager,
};
use colored::Colorize;
use std::path::PathBuf;

/// Handles plugin manager selection, auto-detection and validation.
pub struct PluginManagerService<'a> {
    paths: &'a IrisPaths,
    log: &'a Logger,
}

impl<'a> PluginManagerService<'a> {
    pub fn new(paths: &'a IrisPaths, log: &'a Logger) -> Self {
        Self { paths, log }
    }

    /// Chooses which manager to use based on CLI arguments and logs the result
    pub fn choose(
        &self,
        manager: Option<PluginManager>,
        auto_detect: bool,
    ) -> anyhow::Result<PluginManager> {
        let selected = if auto_detect {
            let detected = self.log.action("Auto-detected plugin manager.", || {
                Ok::<PluginManager, anyhow::Error>(NeovimBridge::detect(self.paths))
            })?;

            self.log.success(&format!(
                "Active manager: {}.",
                detected.to_string().cyan().bold()
            ));
            detected
        } else if let Some(m) = manager {
            self.log.info(&format!(
                "Manual selection: {}.",
                m.to_string().yellow().bold()
            ));
            m
        } else {
            anyhow::bail!("Plugin manager required. Use `--manager <name>` or `--detect`")
        };

        self.validate(&selected)?;

        let count: usize = NeovimBridge::count(self.paths, &selected);
        if count > 0 {
            println!(
                "{} found {} {} {}",
                "└──".dimmed(),
                selected.to_string().bold(),
                format!("({} plugins)", count).dimmed(),
                "✓".green()
            );
        }

        Ok(selected)
    }

    /// Helper to validate the paths for selected plugin manager
    fn validate(&self, manager: &PluginManager) -> anyhow::Result<()> {
        if manager == &PluginManager::Default {
            return Ok(());
        }

        let p: PathBuf = self
            .paths
            .nvim_plugin_path(manager)
            .ok_or_else(|| anyhow::anyhow!("Could not resolve plugin path."))?;

        if !p.exists() {
            anyhow::bail!(
                "Validation failed. {} directory not found at {}.",
                manager,
                p.display().to_string().cyan()
            );
        }

        if !NeovimBridge::has_plugins(&p) {
            anyhow::bail!(
                "Validation failed. {} exists, but no plugins were found in {}.",
                manager,
                p.display().to_string().yellow()
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::IrisContext;

    #[test]
    fn should_error_when_no_manager_and_no_detect() {
        let (_temp, ctx) = IrisContext::mock();
        let service = PluginManagerService::new(&ctx.paths, &ctx.log);

        let result = service.choose(None, false);
        assert!(result.is_err());
    }

    #[test]
    fn should_accept_manual_default_manager_without_validation() {
        let (_temp, ctx) = IrisContext::mock();
        let service = PluginManagerService::new(&ctx.paths, &ctx.log);

        let result = service.choose(Some(PluginManager::Default), false);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), PluginManager::Default);
    }

    #[test]
    fn should_fail_validation_when_manager_dir_missing() {
        let (_temp, ctx) = IrisContext::mock();
        let service = PluginManagerService::new(&ctx.paths, &ctx.log);

        let result = service.choose(Some(PluginManager::Lazy), false);
        assert!(result.is_err());
    }
}
