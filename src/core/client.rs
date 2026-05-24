use crate::{core::IrisPaths, log::Reporter, models::PluginManager};
use anyhow::Result;
use colored::Colorize;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

/// Infrastructure client to interact with Neovim and its filesystem environment
pub struct Client;

impl Client {
    /// Automatically detect plugin manager based on your config
    pub fn detect_manager(paths: &IrisPaths) -> PluginManager {
        if paths.nvim_config_dir().join("lazy-lock.json").exists() {
            return PluginManager::Lazy;
        }

        if Self::has_plugins(&paths.nvim_data_dir().join("lazy")) {
            return PluginManager::Lazy;
        }

        if Self::has_plugins(&paths.nvim_data_dir().join("site/pack/packer/start")) {
            return PluginManager::Packer;
        }

        PluginManager::Default
    }

    /// Validates if manager can be applied
    pub fn validate(paths: &IrisPaths, manager: &PluginManager) -> Result<()> {
        if matches!(manager, PluginManager::Default) {
            return Ok(());
        }

        let p: PathBuf = paths
            .resolve_plugin_path(manager)
            .ok_or_else(|| anyhow::anyhow!("Could not resolve plugin path"))?;

        if !p.exists() {
            anyhow::bail!(
                "Validation failed. {} directory not found at {}",
                manager,
                p.display().to_string().cyan()
            );
        }

        if !Self::has_plugins(&p) {
            anyhow::bail!(
                "Validation failed. {} exists, but no plugins were found in {}",
                manager,
                p.display().to_string().yellow()
            );
        }

        Ok(())
    }

    /// Counts number of folder in plugin directory
    pub fn count_plugins(paths: &IrisPaths, manager: &PluginManager) -> usize {
        paths
            .resolve_plugin_path(manager)
            .and_then(|p| fs::read_dir(p).ok())
            .map(|entries| {
                entries
                    .filter_map(|res| res.ok())
                    .filter(|e| e.path().is_dir())
                    .count()
            })
            .unwrap_or(0)
    }

    /// Get all nvim builtin themes
    pub fn get_builtin_themes() -> Vec<String> {
        let lua_script = r#"
            local rt = vim.fn.expand('$VIMRUNTIME'):gsub('\\', '/')
            local seen = {}
            local builtins = {}

            local function collect(pattern)
                for _, p in ipairs(vim.api.nvim_get_runtime_file(pattern, true)) do
                    local norm = p:gsub('\\', '/')
                    if norm:find(rt, 1, true) then
                        local name = vim.fn.fnamemodify(p, ':t:r')
                        if not seen[name] then
                            seen[name] = true
                            table.insert(builtins, name)
                        end
                    end
                end
            end

            collect('colors/*.vim')
            collect('colors/*.lua')

            table.sort(builtins)
            io.write(table.concat(builtins, ','))
        "#;

        let output = Command::new("nvim")
            .args([
                "--headless",
                "-u",
                "NONE",
                "-c",
                &format!("lua {}", lua_script),
                "-c",
                "qa!",
            ])
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                stdout
                    .split(',')
                    .map(|s| s.trim().to_lowercase())
                    .filter(|s| !s.is_empty())
                    .collect()
            }
            _ => vec![
                "habamax".into(),
                "desert".into(),
                "evening".into(),
                "morning".into(),
                "murphy".into(),
                "pablo".into(),
                "peachpuff".into(),
                "ron".into(),
                "shine".into(),
                "slate".into(),
                "torte".into(),
                "zellner".into(),
            ],
        }
    }

    // Get all themes installed in nvim
    pub fn get_all_themes() -> Result<Vec<String>> {
        let output = Command::new("nvim")
            .args([
                "--headless",
                "-c",
                "lua io.write(table.concat(vim.fn.getcompletion('', 'color'), ','))",
                "+q!",
            ])
            .output()?;

        let s = String::from_utf8_lossy(&output.stdout);
        let mut names: Vec<String> = s
            .split(',')
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty())
            .collect();

        names.sort();
        names.dedup();
        Ok(names)
    }

    /// Helper function to check whether plugins are installed
    fn has_plugins(path: &Path) -> bool {
        fs::read_dir(path)
            .map(|entries| {
                entries
                    .filter_map(|res| res.ok())
                    .any(|e| e.path().is_dir())
            })
            .unwrap_or(false)
    }

    /// Chooses which manager to use based on CLI arguments and logs the result
    pub fn choose(
        paths: &IrisPaths,
        manager: Option<PluginManager>,
        detect: bool,
        log: &Reporter,
    ) -> Result<PluginManager> {
        if detect {
            let res = log.action("Auto-detected plugin manager", || {
                Ok::<PluginManager, anyhow::Error>(Self::detect_manager(paths))
            })?;

            log.success(&format!(
                "Active manager: {}",
                res.to_string().cyan().bold()
            ));

            return Ok(res);
        }

        if let Some(s) = manager {
            log.info(&format!(
                "Manual selection: {}",
                s.to_string().yellow().bold()
            ));
            return Ok(s);
        }

        anyhow::bail!("Plugin manager required. Use `--manager <name>` or `--detect`")
    }
}

/// Unit-tests for nvim client
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::tests::create_test_context;

    #[test]
    fn should_detect_lazy_by_lockfile() {
        let (_temp, ctx) = create_test_context();
        let config_nvim = ctx.paths.nvim_config_dir();

        fs::create_dir_all(&config_nvim).unwrap();
        fs::write(config_nvim.join("lazy-lock.json"), "{}").unwrap();

        let manager: PluginManager = Client::detect_manager(&ctx.paths);
        assert_eq!(manager, PluginManager::Lazy);
    }

    #[test]
    fn should_fail_validation_if_no_plugins_found() {
        let (_temp, ctx) = create_test_context();
        let packer_dir = ctx
            .paths
            .resolve_plugin_path(&PluginManager::Packer)
            .unwrap();

        fs::create_dir_all(&packer_dir).unwrap();

        let res = Client::validate(&ctx.paths, &PluginManager::Packer);
        assert!(res.is_err());
        assert!(
            res.unwrap_err()
                .to_string()
                .contains("no plugins were found")
        );
    }

    #[test]
    fn should_count_plugins_correctly() {
        let (_temp, ctx) = create_test_context();

        let lazy_dir = ctx.paths.resolve_plugin_path(&PluginManager::Lazy).unwrap();
        fs::create_dir_all(lazy_dir.join("p1")).unwrap();
        fs::create_dir_all(lazy_dir.join("p2")).unwrap();
        fs::create_dir_all(lazy_dir.join("p3")).unwrap();

        let count = Client::count_plugins(&ctx.paths, &PluginManager::Lazy);
        assert_eq!(count, 3);
    }

    #[test]
    fn should_return_at_least_basic_builtin_themes() {
        let themes = Client::get_builtin_themes();
        assert!(!themes.is_empty());
        assert!(themes.contains(&"habamax".to_string()));
    }
}
