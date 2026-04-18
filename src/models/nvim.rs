use crate::{core::IrisContext, utils};
use colored::*;
use serde::{Deserialize, Serialize};
use std::{
    env, fmt, fs,
    path::{Path, PathBuf},
};

/// Nvim find theme strategy (plugins)
#[derive(Default, Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
#[serde(tag = "type", content = "value")]
pub enum NvimStrategy {
    #[default]
    Default,

    Lazy,
    Packer,
}

impl NvimStrategy {
    /// Get nvim root path with XDG_DATA_HOME
    fn nvim_data_dir() -> PathBuf {
        let base: PathBuf = env::var("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let home = env::var("HOME").expect("HOME env var not set");
                PathBuf::from(home).join(".local/share")
            });
        base.join("nvim")
    }

    /// Resolve plugins path based on nvim strategy
    pub fn resolve_path(strategy: &NvimStrategy) -> Option<PathBuf> {
        match strategy {
            NvimStrategy::Lazy => Some(Self::nvim_data_dir().join("lazy")),
            NvimStrategy::Packer => Some(Self::nvim_data_dir().join("site/pack/packer/start")),
            NvimStrategy::Default => None,
        }
    }

    /// Automatically detect strategy based on your config
    pub fn detect_strategy() -> NvimStrategy {
        let nvim_data: PathBuf = Self::nvim_data_dir();

        let home: String = env::var("HOME").expect("HOME env var not set");
        let config_dir: PathBuf = env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(&home).join(".config"))
            .join("nvim");

        if config_dir.join("lazy-lock.json").exists() {
            return NvimStrategy::Lazy;
        }

        let lazy_path = nvim_data.join("lazy");
        if Self::has_plugins(&lazy_path) {
            return NvimStrategy::Lazy;
        }

        let packer_path = nvim_data.join("site/pack/packer/start");
        if Self::has_plugins(&packer_path) {
            return NvimStrategy::Packer;
        }

        NvimStrategy::Default
    }

    /// Helper function to check whether plugins are installed
    fn has_plugins(path: &Path) -> bool {
        if !path.is_dir() {
            return false;
        }

        match fs::read_dir(path) {
            Ok(entries) => {
                let count = entries
                    .filter_map(|res| res.ok())
                    .filter(|e| e.path().is_dir())
                    .count();
                count > 0
            }
            Err(_) => false,
        }
    }

    /// Counts number of folder in plugin directory
    pub fn count_plugins(&self) -> usize {
        Self::resolve_path(self)
            .and_then(|p| fs::read_dir(p).ok())
            .map(|entries| {
                entries
                    .filter_map(|res| res.ok())
                    .filter(|e| e.path().is_dir())
                    .count()
            })
            .unwrap_or(0)
    }

    /// Generates Lua-command for runtimepath extension
    pub fn get_rtp_command(&self) -> Option<String> {
        if self.validate().is_err() {
            return None;
        }

        let folder = match self {
            NvimStrategy::Default => return None,
            NvimStrategy::Lazy => "lazy",
            NvimStrategy::Packer => "site/pack/packer/start",
        };

        Some(format!(
            "lua local p = vim.fn.stdpath('data') .. '/{}' for _, dir in ipairs(vim.fn.expand(p .. '/*', false, true)) do vim.opt.rtp:append(dir) end",
            folder
        ))
    }

    /// Validates if strategy can be applied
    pub fn validate(&self) -> anyhow::Result<()> {
        match self {
            NvimStrategy::Default => Ok(()),
            _ => {
                let path: Option<PathBuf> = Self::resolve_path(self);
                let p: PathBuf = path
                    .ok_or_else(|| anyhow::anyhow!("Could not resolve plugin path for {}", self))?;

                if !p.exists() {
                    anyhow::bail!(
                        "Validation failed. {} directory not found at {}",
                        self,
                        utils::pretty_path(&p).cyan()
                    );
                }

                if !Self::has_plugins(&p) {
                    anyhow::bail!(
                        "Validation failed. {} exists, but no plugins were found in {}",
                        self,
                        utils::pretty_path(&p).yellow()
                    );
                }

                Ok(())
            }
        }
    }

    /// Chooses, which strategy to use based on CLI arguments
    pub fn choose(strategy: Option<Self>, detect: bool, ctx: &IrisContext) -> anyhow::Result<Self> {
        if detect {
            let mut d = ctx.log.step("Scanning environment", 1);
            let res = Self::detect_strategy();
            d.done(true);

            println!("      {}  Auto-detected: {}", "󰛔".cyan().bold(), res);
            return Ok(res);
        }

        if let Some(s) = strategy {
            println!("  {}  Manual selection: {}", "󰁕".yellow().bold(), s);
            return Ok(s);
        }

        anyhow::bail!("Strategy required. Use `--strategy <name>` or `--detect`")
    }
}

impl fmt::Display for NvimStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            NvimStrategy::Lazy => "Lazy.nvim".cyan().bold(),
            NvimStrategy::Packer => "Packer.nvim".yellow().bold(),
            NvimStrategy::Default => "Built-in".red().bold(),
        };
        write!(f, "{}", text)
    }
}

/// Unit-tests for nvim strategy
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::tests::create_test_context;

    #[test]
    fn should_detect_lazy_by_lockfile() {
        let (_temp, _) = create_test_context();
        let root = _temp.path();

        temp_env::with_vars(
            [
                ("HOME", Some(root.to_str().unwrap())),
                (
                    "XDG_CONFIG_HOME",
                    Some(root.join(".config").to_str().unwrap()),
                ),
                (
                    "XDG_DATA_HOME",
                    Some(root.join(".local/share").to_str().unwrap()),
                ),
            ],
            || {
                let config_nvim = root.join(".config/nvim");
                fs::create_dir_all(&config_nvim).unwrap();
                fs::write(config_nvim.join("lazy-lock.json"), "{}").unwrap();

                let strategy = NvimStrategy::detect_strategy();
                assert_eq!(strategy, NvimStrategy::Lazy);
            },
        );
    }

    #[test]
    fn should_fail_validation_if_no_plugins_found() {
        let (_temp, _) = create_test_context();
        let root = _temp.path();

        temp_env::with_var("XDG_DATA_HOME", Some(root.join(".local/share")), || {
            let packer_dir = root.join(".local/share/nvim/site/pack/packer/start");
            fs::create_dir_all(&packer_dir).unwrap();

            let strategy = NvimStrategy::Packer;
            let res = strategy.validate();

            assert!(res.is_err());
            assert!(
                res.unwrap_err()
                    .to_string()
                    .contains("no plugins were found")
            );
        });
    }

    #[test]
    fn should_get_rtp_command_only_after_validation() {
        let (_temp, _) = create_test_context();
        let root = _temp.path();

        temp_env::with_var("XDG_DATA_HOME", Some(root.join(".local/share")), || {
            let strategy = NvimStrategy::Lazy;
            assert!(strategy.get_rtp_command().is_none());

            let plugin_dir = root.join(".local/share/nvim/lazy/some_plugin");
            fs::create_dir_all(plugin_dir).unwrap();

            let cmd = strategy.get_rtp_command();
            assert!(cmd.is_some());
            assert!(cmd.unwrap().contains("rtp:append"));
        });
    }

    #[test]
    fn should_count_plugins_correctly() {
        let (_temp, _) = create_test_context();
        let root = _temp.path();

        temp_env::with_var("XDG_DATA_HOME", Some(root.join(".local/share")), || {
            let strategy = NvimStrategy::Lazy;
            let lazy_dir = root.join(".local/share/nvim/lazy");

            fs::create_dir_all(lazy_dir.join("p1")).unwrap();
            fs::create_dir_all(lazy_dir.join("p2")).unwrap();
            fs::create_dir_all(lazy_dir.join("p3")).unwrap();

            assert_eq!(strategy.count_plugins(), 3);
        });
    }
}
