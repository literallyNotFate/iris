use anyhow::{Context, Result, anyhow};
use std::path::PathBuf;

/// Paths manager for application
#[derive(Clone)]
pub struct IrisPaths {
    /// Points to: ~/.config/iris
    pub config: PathBuf,
    /// Points to: ~/.cache/iris
    pub cache: PathBuf,
    /// Points to: ~/.config/iris/state.json
    pub state_file: PathBuf,
    /// Points to: ~/.cache/iris/current_theme
    pub current_theme: PathBuf,
}

impl IrisPaths {
    pub fn new() -> Result<Self> {
        let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not find home directory"))?;

        let config = std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| home.join(".config"))
            .join("iris");

        let cache = std::env::var("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| home.join(".cache"))
            .join("iris");

        Ok(Self {
            state_file: config.join("state.json"),
            current_theme: cache.join("current_theme"),
            config,
            cache,
        })
    }

    /// Creates all folders for iris if there none
    pub fn ensure_dirs(&self) -> Result<()> {
        std::fs::create_dir_all(&self.config)
            .with_context(|| "Failed to create config directory")?;
        std::fs::create_dir_all(&self.cache).with_context(|| "Failed to create cache directory")?;
        Ok(())
    }
}

/// Unit-tests context paths
#[cfg(test)]
mod tests {
    use super::*;
    use tempdir::TempDir;

    #[test]
    fn should_handle_paths_creation_with_xdg() {
        let temp_dir: TempDir = TempDir::new("iris_paths_test").expect("Failed to create temp dir");
        let fake_config: PathBuf = temp_dir.path().join("my_config");
        let fake_cache: PathBuf = temp_dir.path().join("my_cache");

        temp_env::with_vars(
            vec![
                ("XDG_CONFIG_HOME", Some(fake_config.to_str().unwrap())),
                ("XDG_CACHE_HOME", Some(fake_cache.to_str().unwrap())),
            ],
            || {
                let paths = IrisPaths::new().expect("Should create paths");

                let expected_config_base: PathBuf = fake_config.join("iris");
                let expected_cache_base: PathBuf = fake_cache.join("iris");

                assert_eq!(paths.config, expected_config_base);
                assert_eq!(paths.cache, expected_cache_base);
                assert_eq!(paths.state_file, expected_config_base.join("state.json"));
                assert_eq!(
                    paths.current_theme,
                    expected_cache_base.join("current_theme")
                );
            },
        );
    }

    #[test]
    fn should_create_folders_with_ensure_dirs() {
        let temp_dir: TempDir =
            TempDir::new("iris_ensure_test").expect("Failed to create temp dir");
        let base = temp_dir.path();

        let paths = IrisPaths {
            config: base.join("config"),
            cache: base.join("cache"),
            state_file: base.join("config/state.json"),
            current_theme: base.join("cache/current_theme"),
        };

        assert!(!paths.config.exists());

        paths.ensure_dirs().expect("Failed to ensure directories");

        assert!(paths.config.is_dir());
        assert!(paths.cache.is_dir());
    }
}
