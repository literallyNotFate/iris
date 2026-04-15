use anyhow::{Context, Result, anyhow};
use std::path::PathBuf;

/// Paths manager for application
#[derive(Clone)]
pub struct IrisPaths {
    pub config: PathBuf, // ~/.config/iris
    pub cache: PathBuf,  // ~/.cache/iris

    pub core: PathBuf,       // ~/.cache/iris/core (state, palette)
    pub generators: PathBuf, // ~/.cache/iris/generators (application)
    pub bin: PathBuf,        // ~/.cache/iris/bin (fzf scripts etc)

    pub state_file: PathBuf,    // ~/.config/iris/state.json
    pub current_theme: PathBuf, // ~/.cache/iris/core/current_theme
    pub palettes: PathBuf,      // ~/.cache/iris/core/palettes
}

impl IrisPaths {
    pub fn new() -> Result<Self> {
        let home: PathBuf =
            dirs::home_dir().ok_or_else(|| anyhow!("Could not find home directory"))?;

        let config: PathBuf = std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| home.join(".config"))
            .join("iris");

        let cache: PathBuf = std::env::var("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| home.join(".cache"))
            .join("iris");

        let core: PathBuf = cache.join("core");
        let generators: PathBuf = cache.join("gen");
        let bin: PathBuf = cache.join("bin");

        Ok(Self {
            state_file: config.join("state.json"),
            current_theme: core.join("current_theme"),
            palettes: core.join("palettes"),
            config,
            cache,
            core,
            generators,
            bin,
        })
    }

    /// Creates all folders for iris if there none
    pub fn ensure_dirs(&self) -> Result<()> {
        std::fs::create_dir_all(&self.config)
            .with_context(|| "Failed to create config directory")?;
        std::fs::create_dir_all(&self.core).with_context(|| "Failed to create core directory")?;
        std::fs::create_dir_all(&self.palettes).with_context(|| "Failed to create palette path")?;
        std::fs::create_dir_all(&self.generators)
            .with_context(|| "Failed to create generators directory")?;
        std::fs::create_dir_all(&self.bin).with_context(|| "Failed to create bin directory")?;
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
        let root = temp_dir.path();
        let fake_config: PathBuf = root.join("my_config");
        let fake_cache: PathBuf = root.join("my_cache");

        temp_env::with_vars(
            vec![
                ("HOME", Some(root.to_str().unwrap())),
                ("XDG_CONFIG_HOME", Some(fake_config.to_str().unwrap())),
                ("XDG_CACHE_HOME", Some(fake_cache.to_str().unwrap())),
            ],
            || {
                let paths = IrisPaths::new().expect("Should create paths");

                let expected_config_base: PathBuf = fake_config.join("iris");
                let expected_cache_base: PathBuf = fake_cache.join("iris");

                assert_eq!(paths.config, expected_config_base, "Config path mismatch");
                assert_eq!(paths.cache, expected_cache_base, "Cache path mismatch");
                assert_eq!(
                    paths.state_file,
                    expected_config_base.join("state.json"),
                    "State file path mismatch"
                );
                assert_eq!(
                    paths.current_theme,
                    expected_cache_base.join("core").join("current_theme"),
                    "Current theme path mismatch"
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
            core: base.join("cache/core"),
            generators: base.join("cache/generators"),
            bin: base.join("cache/bin"),
            state_file: base.join("config/state.json"),
            current_theme: base.join("cache/core/current_theme"),
            palettes: base.join("cache/core/palettes"),
        };

        assert!(!paths.config.exists());
        assert!(!paths.core.exists());
        assert!(!paths.palettes.exists());

        paths.ensure_dirs().expect("Failed to ensure directories");

        assert!(paths.config.is_dir());
        assert!(paths.cache.is_dir());
        assert!(paths.core.is_dir());
        assert!(paths.palettes.is_dir());
        assert!(paths.generators.is_dir());
        assert!(paths.bin.is_dir());
    }
}
