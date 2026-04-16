use anyhow::{Context, Result};
use std::{fs, path::PathBuf};

/// Paths manager for application
#[derive(Clone)]
pub struct IrisPaths {
    pub config: PathBuf, // ~/.config/iris
    pub cache: PathBuf,  // ~/.cache/iris

    pub core: PathBuf,       // ~/.cache/iris/core (state, palette)
    pub generators: PathBuf, // ~/.cache/iris/gen (application)
    pub bin: PathBuf,        // ~/.cache/iris/bin (fzf scripts etc)

    pub state_file: PathBuf,    // ~/.config/iris/state.json
    pub current_theme: PathBuf, // ~/.cache/iris/core/current_theme
    pub palettes: PathBuf,      // ~/.cache/iris/core/palettes
}

impl IrisPaths {
    pub fn new() -> Result<Self> {
        let home: PathBuf = dirs::home_dir().with_context(|| "Could not find home directory")?;

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
        fs::create_dir_all(&self.config).context("Failed to create 'config' directory")?;
        fs::create_dir_all(&self.core).context("Failed to create 'core' directory")?;
        fs::create_dir_all(&self.palettes).context("Failed to create palette path")?;
        fs::create_dir_all(&self.generators).context("Failed to create 'gen' directory")?;
        fs::create_dir_all(&self.bin).context("Failed to create 'bin' directory")?;
        Ok(())
    }

    /// Cleans only 'gen' folder, where all themes located (e.g bat.conf)
    pub fn clean_gen(&self) -> Result<()> {
        if self.generators.exists() {
            fs::remove_dir_all(&self.generators).context("Cannot remove the 'gen' folder")?;
        }
        if self.bin.exists() {
            fs::remove_dir_all(&self.bin).context("Cannot remove the 'bin' folder")?;
        }

        fs::create_dir_all(&self.generators).context("Failed to recreate 'gen' directory")?;
        fs::create_dir_all(&self.bin).context("Failed to recreate 'bin' directory")?;
        Ok(())
    }

    /// Clears the entire iris cache directory and recreates empty directories
    pub fn purge_all(&self) -> Result<()> {
        if self.cache.exists() {
            fs::remove_dir_all(&self.cache).context("Cannot clear the 'cache' directory")?;
        }

        self.ensure_dirs()?;
        Ok(())
    }
}

/// Unit-tests context paths
#[cfg(test)]
mod tests {
    use super::*;
    use tempdir::TempDir;

    // Helper function to setup paths
    fn setup_paths() -> IrisPaths {
        let temp_dir: TempDir = TempDir::new("iris_paths_test").expect("Failed to create temp dir");
        let base = temp_dir.path();

        IrisPaths {
            config: base.join("config"),
            cache: base.join("cache"),
            core: base.join("cache/core"),
            generators: base.join("cache/generators"),
            bin: base.join("cache/bin"),
            state_file: base.join("config/state.json"),
            current_theme: base.join("cache/core/current_theme"),
            palettes: base.join("cache/core/palettes"),
        }
    }

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
        let paths: IrisPaths = setup_paths();

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

    #[test]
    fn should_clean_gen_folder_with_bin() {
        let paths: IrisPaths = setup_paths();
        let gen_path = paths.cache.join("gen");
        let gen_dummy_file = gen_path.join("bat/bat.conf");
        let bin_dummy_file = paths.bin.join("fzf.sh");

        fs::create_dir_all(gen_dummy_file.parent().unwrap()).unwrap();
        fs::create_dir_all(bin_dummy_file.parent().unwrap()).unwrap();
        fs::write(&gen_dummy_file, "test").unwrap();
        fs::write(&bin_dummy_file, "test").unwrap();
        assert!(gen_dummy_file.exists());
        assert!(bin_dummy_file.exists());

        paths.clean_gen().expect("Clean gen failed");

        assert!(
            paths.cache.exists(),
            "Root cache directory should still exist"
        );
        assert!(
            gen_path.exists(),
            "Generators directory should be recreated"
        );
        assert!(paths.bin.exists(), "Bin directory should be recreated");

        let gen_entries = fs::read_dir(&paths.generators).unwrap().count();
        assert_eq!(gen_entries, 0, "Generators directory should be empty");

        let bin_entries = fs::read_dir(&paths.bin).unwrap().count();
        assert_eq!(bin_entries, 0, "Bin directory should be empty");
    }

    #[test]
    fn should_purge_all() {
        let paths = setup_paths();
        let theme_file = paths.cache.join("themes/melange.tmTheme");
        let gen_file = paths.cache.join("gen/bat/bat.conf");

        fs::create_dir_all(theme_file.parent().unwrap()).unwrap();
        fs::create_dir_all(gen_file.parent().unwrap()).unwrap();
        fs::write(&theme_file, "data").unwrap();
        fs::write(&gen_file, "data").unwrap();

        paths.purge_all().expect("Purge all failed");
        assert!(paths.cache.exists(), "Cache root should be recreated");

        let entries = fs::read_dir(&paths.cache).unwrap().count();
        assert_eq!(
            entries, 3,
            "Cache should contain exactly 3 base directories (core, gen, bin)"
        );

        assert!(!theme_file.exists(), "Old theme file should be gone");

        let gen_entries = fs::read_dir(&paths.generators).unwrap().count();
        assert_eq!(gen_entries, 0, "Generators directory should be empty");
        assert!(
            paths.config.exists(),
            "Config directory should never be purged"
        );
    }
}
