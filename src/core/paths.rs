use crate::models::PluginManager;
use anyhow::{Context, Result};
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Paths manager for application
#[derive(Clone)]
pub struct IrisPaths {
    pub config: PathBuf, // ~/.config/iris
    pub cache: PathBuf,  // ~/.cache/iris

    pub core: PathBuf,       // ~/.cache/iris/core (state, palette)
    pub generators: PathBuf, // ~/.cache/iris/gen (application)
    pub bin: PathBuf,        // ~/.cache/iris/bin (fzf scripts etc)

    pub state_file: PathBuf,    // ~/.config/iris/state.toml
    pub current_theme: PathBuf, // ~/.cache/iris/core/current_theme
    pub themes: PathBuf,        // ~/.cache/iris/core/themes
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
            state_file: config.join("state.toml"),
            current_theme: core.join("current_theme"),
            themes: core.join("themes"),
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
        fs::create_dir_all(&self.themes).context("Failed to create themes path")?;
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

    /// Recursively calculates the size of requested directory in bytes
    pub fn get_size(&self, path: &Path) -> u64 {
        let metadata = match path.metadata() {
            Ok(m) => m,
            Err(_) => return 0,
        };

        if metadata.is_file() {
            return metadata.len();
        }

        if metadata.is_dir() {
            if let Ok(entries) = fs::read_dir(path) {
                return entries
                    .flatten()
                    .map(|entry| self.get_size(&entry.path()))
                    .sum();
            }
        }

        0
    }

    /// Get Neovim root path with XDG_DATA_HOME
    pub fn nvim_data_dir(&self) -> PathBuf {
        if cfg!(test) {
            self.cache.join("nvim_data")
        } else {
            std::env::var("XDG_DATA_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| dirs::home_dir().unwrap_or_default().join(".local/share"))
                .join("nvim")
        }
    }

    /// Get system path for Neovim config (~/.config/nvim)
    pub fn nvim_config_dir(&self) -> PathBuf {
        if cfg!(test) {
            self.config.join("nvim_config")
        } else {
            std::env::var("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| dirs::home_dir().unwrap_or_default().join(".config"))
                .join("nvim")
        }
    }

    /// Resolves plugin manager relative path to absolute
    pub fn resolve_plugin_path(&self, manager: &PluginManager) -> Option<PathBuf> {
        manager
            .plugin_subdirectory()
            .map(|sub| self.nvim_data_dir().join(sub))
    }

    /// Returns absolute path to JSON theme cache
    pub fn cached_theme(&self, theme_name: &str) -> PathBuf {
        self.themes
            .join(theme_name.to_lowercase())
            .with_extension("json")
    }

    /// Checks whether requested theme is already cached
    pub fn is_theme_cached(&self, name: &str) -> bool {
        self.cached_theme(name).exists()
    }

    /// Returns a sorted list of theme names available in cache
    pub fn get_cached_themes(&self) -> Result<Vec<String>> {
        let mut themes: Vec<String> = fs::read_dir(&self.themes)?
            .filter_map(|entry| {
                let path = entry.ok()?.path();

                if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
                    Some(path.file_stem()?.to_string_lossy().into_owned())
                } else {
                    None
                }
            })
            .collect();

        themes.sort();
        Ok(themes)
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
            state_file: base.join("config/state.toml"),
            current_theme: base.join("cache/core/current_theme"),
            themes: base.join("cache/core/themes"),
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
                    expected_config_base.join("state.toml"),
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
        assert!(!paths.themes.exists());

        paths.ensure_dirs().expect("Failed to ensure directories");

        assert!(paths.config.is_dir());
        assert!(paths.cache.is_dir());
        assert!(paths.core.is_dir());
        assert!(paths.themes.is_dir());
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

    #[test]
    fn should_calculate_size_of_directory() {
        let paths = setup_paths();
        let root = paths.cache.to_path_buf();
        fs::create_dir_all(&root).unwrap();

        let empty_file = root.join("empty.txt");
        fs::write(&empty_file, "").unwrap();
        assert_eq!(paths.get_size(&empty_file), 0);

        let data_file = root.join("data.txt");
        fs::write(&data_file, "hello world").unwrap();
        assert_eq!(paths.get_size(&data_file), 11);

        let dir = root.join("folder");
        fs::create_dir(&dir).unwrap();
        fs::write(dir.join("a.txt"), "abc").unwrap();

        let sub = dir.join("sub");
        fs::create_dir(&sub).unwrap();
        fs::write(sub.join("b.txt"), "de").unwrap();

        assert_eq!(paths.get_size(&dir), 5);
        assert_eq!(paths.get_size(&root.join("missing")), 0);
    }

    #[test]
    fn should_handle_theme_cached_case_insensitivity() {
        let paths = setup_paths();
        let palettes_dir = paths.themes.to_path_buf();
        fs::create_dir_all(&palettes_dir).unwrap();
        fs::write(palettes_dir.join("gruvbox.json"), "{}").unwrap();

        assert!(paths.is_theme_cached("gruvbox"));
        assert!(paths.is_theme_cached("Gruvbox"));
        assert!(paths.is_theme_cached("GRUVBOX"));
        assert!(!paths.is_theme_cached("nord"));
    }

    #[test]
    fn should_return_all_cached_themes() {
        let paths = setup_paths();
        let palettes_dir = paths.themes.to_path_buf();
        fs::create_dir_all(&palettes_dir).unwrap();

        fs::write(palettes_dir.join("nord.json"), "{}").unwrap();
        fs::write(palettes_dir.join("gruvbox.json"), "{}").unwrap();
        fs::write(palettes_dir.join("tokyonight.json"), "{}").unwrap();

        fs::write(palettes_dir.join("README.md"), "").unwrap();
        fs::write(palettes_dir.join(".DS_Store"), "").unwrap();
        fs::create_dir(palettes_dir.join("subfolder.json")).unwrap();

        let themes = paths.get_cached_themes().unwrap();
        assert_eq!(themes.len(), 3);

        assert_eq!(themes[0], "gruvbox");
        assert_eq!(themes[1], "nord");
        assert_eq!(themes[2], "tokyonight");
        assert!(!themes.contains(&"README".to_string()));
        assert!(!themes.contains(&"subfolder".to_string()));
    }

    #[test]
    fn should_return_theme_cache_path() {
        let paths = setup_paths();
        let expected = paths.cache.join("core/themes/melange.json");
        let path = paths.cached_theme("melange");
        assert_eq!(path, expected);
    }
}
