use super::IrisPaths;
use crate::{
    infra::RESOURCES_DIR,
    models::{Palette, PluginManager, State},
};
use std::{fs, path::Path, process::Command};

/// Infrastructure client to interact with Neovim and its filesystem environment
pub struct NeovimBridge;

impl NeovimBridge {
    /// Automatically detect plugin manager based on your config
    pub fn detect(paths: &IrisPaths) -> PluginManager {
        if paths.nvim_config_path().join("lazy-lock.json").exists() {
            return PluginManager::Lazy;
        }

        if Self::has_plugins(&paths.nvim_data_path().join("lazy")) {
            return PluginManager::Lazy;
        }

        if Self::has_plugins(&paths.nvim_data_path().join("site/pack/packer/start")) {
            return PluginManager::Packer;
        }

        PluginManager::Default
    }

    /// Counts number of folders in plugin directory
    pub fn count(paths: &IrisPaths, manager: &PluginManager) -> usize {
        paths
            .nvim_plugin_path(manager)
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
    pub fn builtin_themes() -> Vec<String> {
        let script: &str = RESOURCES_DIR
            .get_file("lua/get_builtin_themes.lua")
            .expect("get_builtin_themes.lua must be included")
            .contents_utf8()
            .expect("File must be valid utf8");

        let output = Command::new("nvim")
            .args([
                "--headless",
                "-u",
                "NONE",
                "-c",
                &format!("lua {}", script),
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

    /// Get all themes installed in nvim
    pub fn installed_themes() -> anyhow::Result<Vec<String>> {
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
    pub(crate) fn has_plugins(path: &Path) -> bool {
        fs::read_dir(path)
            .map(|entries| {
                entries
                    .filter_map(|res| res.ok())
                    .any(|e| e.path().is_dir())
            })
            .unwrap_or(false)
    }

    /// Runs Neovim in headless mode to capture the colors of the selected theme
    pub fn run_fetch_bridge(theme: &str, state: &State) -> anyhow::Result<String> {
        let mut args: Vec<String> = Self::build_base_args(state);
        args.extend([
            "-c".into(),
            format!("colorscheme {}", theme.to_lowercase()),
            "-c".into(),
            format!("lua {}", Palette::fetch_lua_script()),
            "-c".into(),
            "qa!".into(),
        ]);

        let output = Command::new("nvim").args(&args).output()?;
        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("`nvim` failed to export palette: {}.", error_msg.trim());
        }

        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    /// Checks whether Neovim can apply theme without any errors
    pub fn check_theme_exists(theme: &str, state: &State) -> bool {
        let mut args: Vec<String> = Self::build_base_args(state);
        args.extend([
            "-c".into(),
            format!(
                "try | colorscheme {} | qa! | catch | cquit 1 | endtry",
                theme.to_lowercase()
            ),
        ]);

        match Command::new("nvim").args(&args).output() {
            Ok(o) => o.status.success(),
            _ => false,
        }
    }

    /// Helper to build flags for Neovim
    fn build_base_args(state: &State) -> Vec<String> {
        let mut args: Vec<String> = vec!["--headless".to_string()];
        args.push("-u".into());
        args.push("NONE".into());

        if state.nvim.manager != PluginManager::Default {
            if let Some(rtp_cmd) = state.get_rtp_command() {
                args.push("-c".into());
                args.push(rtp_cmd);
            }
        }
        args
    }
}

/// Unit-tests for nvim client
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::IrisContext;

    #[test]
    fn should_detect_lazy_by_lockfile() {
        let (_temp, ctx) = IrisContext::mock();
        let config_nvim = ctx.paths.nvim_config_path();

        fs::create_dir_all(&config_nvim).unwrap();
        fs::write(config_nvim.join("lazy-lock.json"), "{}").unwrap();

        let manager: PluginManager = NeovimBridge::detect(&ctx.paths);
        assert_eq!(manager, PluginManager::Lazy);
    }

    #[test]
    fn should_count_plugins_correctly() {
        let (_temp, ctx) = IrisContext::mock();

        let lazy_dir = ctx.paths.nvim_plugin_path(&PluginManager::Lazy).unwrap();
        fs::create_dir_all(lazy_dir.join("p1")).unwrap();
        fs::create_dir_all(lazy_dir.join("p2")).unwrap();
        fs::create_dir_all(lazy_dir.join("p3")).unwrap();

        let count = NeovimBridge::count(&ctx.paths, &PluginManager::Lazy);
        assert_eq!(count, 3);
    }

    #[test]
    fn should_return_at_least_basic_builtin_themes() {
        let themes = NeovimBridge::builtin_themes();
        assert!(!themes.is_empty());
        assert!(themes.contains(&"habamax".to_string()));
    }

    #[test]
    fn should_test_build_args_for_lazy_manager() {
        let (_temp, mut ctx) = IrisContext::mock();
        ctx.state.nvim.manager = PluginManager::Lazy;
        let args = NeovimBridge::build_base_args(&ctx.state);

        assert!(args.contains(&"--headless".to_string()));
        assert!(args.contains(&"NONE".to_string()));

        let has_rtp = args.iter().any(|a| a.contains("vim.opt.rtp:append"));
        assert!(
            has_rtp,
            "Lazy manager must include RTP setup in base arguments"
        );
    }

    #[test]
    fn should_test_build_args_without_rtp_for_default_manager() {
        let (_temp, mut ctx) = IrisContext::mock();
        ctx.state.nvim.manager = PluginManager::Default;
        let args = NeovimBridge::build_base_args(&ctx.state);

        let has_rtp = args.iter().any(|a| a.contains("vim.opt.rtp:append"));
        assert!(!has_rtp, "Default manager should NOT include RTP setup");
    }
}
