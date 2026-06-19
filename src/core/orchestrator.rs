use crate::{
    core::{IrisPaths, NeovimBridge},
    log::Logger,
    models::{Palette, PluginManager, State, Theme},
    utils,
};
use anyhow::{Context, Error, Result};
use colored::Colorize;
use serde::Deserialize;
use std::{fs, path::PathBuf};

/// Environment Neovim theme orchestrator that basically provides plugin manager
/// selection w/validation and theme collection
pub struct ThemeOrchestrator<'a> {
    paths: &'a IrisPaths,
    log: &'a Logger,
}

impl<'a> ThemeOrchestrator<'a> {
    pub fn new(paths: &'a IrisPaths, log: &'a Logger) -> Self {
        Self { paths, log }
    }

    /// Gets the name of the current theme from Iris cache file
    pub fn get_current_theme(&self) -> Result<String> {
        let path: PathBuf = self.paths.current_theme.to_path_buf();
        let content: String = fs::read_to_string(&path).map_err(|_| {
            anyhow::anyhow!(format!(
                "No active theme detected.\n\
                    {}: Make sure to switch theme in Neovim or pass the name manually: `{}`",
                "Tip".bold().cyan(),
                "iris switch <name>".bold().cyan()
            ))
        })?;

        let trimmed: &str = content.trim();
        if trimmed.is_empty() {
            anyhow::bail!("Theme cache is empty");
        }

        Ok(utils::capitalize(trimmed))
    }

    /// Get theme sync status
    pub fn get_sync_status(&self, state: &State) -> (String, bool) {
        let current: &String = &state.current_theme;
        let nvim_theme: String = self.get_current_theme().unwrap_or_default();
        let is_sync: bool = nvim_theme.eq_ignore_ascii_case(current);

        let display_name: String = if is_sync {
            utils::capitalize(current)
        } else if nvim_theme.is_empty() {
            "Unknown".to_string()
        } else {
            utils::capitalize(&nvim_theme)
        };

        (display_name, is_sync)
    }

    /// Load theme (with cache check, validation and logging)
    pub fn load_theme(
        &self,
        theme_name: &str,
        force: bool,
        save: bool,
        state: &State,
    ) -> Result<Theme> {
        let theme_lower: String = theme_name.to_lowercase();
        let theme_cap: String = utils::capitalize(theme_name);
        let cache_path: PathBuf = self.paths.cached_theme(&theme_name);

        let main_task = self.log.step_with_icon(
            "".magenta().bold(),
            &format!("Fetching: {}", theme_cap.cyan().bold()),
            true,
        );

        if !force && cache_path.exists() {
            if let Ok(cached_theme) = Theme::load_from_cache(&cache_path) {
                main_task.log.info(&format!(
                    "Using cached theme for {}...",
                    theme_cap.yellow().bold()
                ));
                main_task.done();
                return Ok(cached_theme);
            }
        }

        if state.manager == PluginManager::Default {
            let builtins: Vec<String> = NeovimBridge::get_builtin_themes();
            if !builtins.contains(&theme_lower) {
                if cache_path.exists() {
                    if let Ok(cached_theme) = Theme::load_from_cache(&cache_path) {
                        main_task
                            .log
                            .warn("Built-in mode active. Using existing cache for external theme.");
                        main_task.done();
                        return Ok(cached_theme);
                    }
                }
                anyhow::bail!(
                    "Theme `{}` is not a built-in theme and not cached",
                    theme_name.yellow().bold()
                );
            }
        }

        if force {
            main_task.log.info(&format!(
                "`{}` flag detected. Bypassing cache...",
                "--force".cyan()
            ));
        }

        let stdout: String = main_task.log.action("Executed Lua bridge", || {
            NeovimBridge::run_fetch_bridge(theme_name, state)
        })?;

        let palette: Palette = main_task
            .log
            .action("Parsed palette data", || Self::parse_nvim_stdout(&stdout))?;

        let theme_obj: Theme = Theme::new(&theme_cap, palette);
        if save {
            main_task.log.action("Saved theme to cache", || {
                theme_obj.save_to_cache(&cache_path)
            })?;
        }

        main_task.done_with(&format!(
            "Theme `{}` fetched successfully!",
            theme_cap.yellow()
        ));

        Ok(theme_obj)
    }

    /// Checks if theme exists (in cache, neovim or builtin)
    pub fn theme_exists(&self, theme_name: &str, state: &State) -> bool {
        let theme_lower: String = theme_name.to_lowercase();

        if which::which("nvim").is_err() {
            return false;
        }

        if self.paths.is_theme_cached(&theme_lower) {
            return true;
        }

        if state.manager == PluginManager::Default {
            return NeovimBridge::get_builtin_themes().contains(&theme_lower);
        }

        NeovimBridge::check_theme_exists(&theme_lower, state)
    }

    /// Helper to clear stdout from Neovim garbage and extract JSON palette
    fn parse_nvim_stdout(stdout: &str) -> Result<Palette> {
        let json_start: usize = stdout
            .find('{')
            .context("Failed to locate opening brace '{' of palette JSON within Neovim output")?;

        let mut deserializer = serde_json::Deserializer::from_str(&stdout[json_start..]);
        let palette = Palette::deserialize(&mut deserializer)
            .context("Failed to parse palette JSON within Neovim output")?;

        Ok(palette)
    }

    /// Chooses which manager to use based on CLI arguments and logs the result
    pub fn choose_manager(
        &self,
        manual_manager: Option<PluginManager>,
        auto_detect: bool,
    ) -> Result<PluginManager> {
        let selected = if auto_detect {
            let detected = self.log.action("Auto-detecting plugin manager", || {
                Ok::<PluginManager, Error>(NeovimBridge::detect_manager(self.paths))
            })?;

            self.log.success(&format!(
                "Active manager: {}",
                detected.to_string().cyan().bold()
            ));
            detected
        } else if let Some(m) = manual_manager {
            self.log.info(&format!(
                "Manual selection: {}",
                m.to_string().yellow().bold()
            ));
            m
        } else {
            anyhow::bail!("Plugin manager required. Use `--manager <name>` or `--detect`")
        };

        self.validate_manager(&selected)?;

        let count: usize = NeovimBridge::count_plugins(self.paths, &selected);
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

    /// Get all themes
    pub fn get_themes(&self) -> Vec<String> {
        match NeovimBridge::get_all_themes() {
            Ok(themes) if !themes.is_empty() => themes,
            _ => {
                self.log.warn(
                    "Could not fetch themes from active Neovim session. Falling back to builtins.",
                );
                NeovimBridge::get_builtin_themes()
            }
        }
    }

    /// Helper to validate the paths for selected plugin manager
    fn validate_manager(&self, manager: &PluginManager) -> Result<()> {
        if manager == &PluginManager::Default {
            return Ok(());
        }

        let p: PathBuf = self
            .paths
            .resolve_plugin_path(manager)
            .ok_or_else(|| anyhow::anyhow!("Could not resolve plugin path"))?;

        if !p.exists() {
            anyhow::bail!(
                "Validation failed. {} directory not found at {}",
                manager,
                p.display().to_string().cyan()
            );
        }

        if !NeovimBridge::has_plugins(&p) {
            anyhow::bail!(
                "Validation failed. {} exists, but no plugins were found in {}",
                manager,
                p.display().to_string().yellow()
            );
        }

        Ok(())
    }
}

/// Unit-tests for environment
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::tests::create_test_context;

    /// Helper to create dummy json palette for tests
    fn make_dummy_palette_json() -> &'static str {
        r##"{"bg":"#ffffff","fg":"","caret":"","line_hl":"","sel":"","gutter_fg":"","comment":"","variable":"","constant":"","number":"","string":"","keyword":"","operator":"","func":"","type_name":"","tag":"","attribute":"","white":"","ansi":[]}"##
    }

    #[test]
    fn should_save_and_load_cache() {
        let (_temp, ctx) = create_test_context();
        let orchestrator = ThemeOrchestrator::new(&ctx.paths, &ctx.log);
        let theme_name = "catppuccin";
        let cache_path = ctx.paths.themes.join(format!("{}.json", theme_name));
        let dummy_palette: Palette = serde_json::from_str(make_dummy_palette_json()).unwrap();
        let theme_to_cache = Theme::new(&utils::capitalize(theme_name), dummy_palette);

        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(
            &cache_path,
            serde_json::to_string_pretty(&theme_to_cache).unwrap(),
        )
        .unwrap();
        assert!(cache_path.exists());

        let result = orchestrator.load_theme(theme_name, false, false, &ctx.state);
        assert!(result.is_ok());

        let loaded_theme = result.unwrap();
        assert_eq!(loaded_theme.name, utils::capitalize(theme_name));
        assert_eq!(loaded_theme.colors.bg, "#ffffff");
    }

    #[test]
    fn should_read_theme_from_valid_path() {
        let (_temp, ctx) = create_test_context();
        let orchestrator = ThemeOrchestrator::new(&ctx.paths, &ctx.log);
        let cache_path = &ctx.paths.current_theme;

        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&cache_path, "  melange  ").unwrap();

        let result = orchestrator.get_current_theme().unwrap();
        assert_eq!(result, "Melange");
    }

    #[test]
    fn should_invoke_error_when_theme_file_is_empty() {
        let (temp, ctx) = create_test_context();
        let cache_path: PathBuf = temp.path().join("empty_theme");
        let orchestrator = ThemeOrchestrator::new(&ctx.paths, &ctx.log);

        fs::write(&cache_path, "    ").unwrap();

        let result = orchestrator.get_current_theme();
        assert!(result.is_err());
    }

    #[test]
    fn should_parse_nvim_json_with_garbage() {
        let raw_output = r##"
            [NVIM] Warning: Semantic tokens not supported
            {
                "bg": "#121212", "fg": "#ffffff", "caret": "#ffffff",
                "line_hl": "#000000", "sel": "#000000", "gutter_fg": "#000000",
                "comment": "#000000", "variable": "#000000", "constant": "#000000",
                "number": "#000000", "string": "#000000", "keyword": "#000000",
                "operator": "#000000", "func": "#000000", "type_name": "#000000",
                "tag": "#000000", "attribute": "#000000", "white": "#ffffff",
                "ansi": [
                    "#000000", "#111111", "#222222", "#333333",
                    "#444444", "#555555", "#666666", "#777777",
                    "#888888", "#999999", "#aaaaaa", "#bbbbbb",
                    "#cccccc", "#dddddd", "#eeeeee", "#ffffff"
                ]
            }
            [NVIM] Process exited
        "##;

        let result = ThemeOrchestrator::parse_nvim_stdout(raw_output);
        assert!(
            result.is_ok(),
            "Parser failed with error: {:?}",
            result.err()
        );
        assert_eq!(result.unwrap().bg, "#121212");
    }

    #[test]
    fn should_read_from_cache_in_default_manager_even_if_external() {
        let (_temp, mut ctx) = create_test_context();
        ctx.state.manager = PluginManager::Default;
        let orchestrator = ThemeOrchestrator::new(&ctx.paths, &ctx.log);
        let theme = "vesper";
        let cache_path = ctx.paths.themes.join(format!("{}.json", theme));
        let dummy_palette: Palette = serde_json::from_str(make_dummy_palette_json()).unwrap();
        let theme_to_cache = Theme::new("Vesper", dummy_palette);

        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(
            &cache_path,
            serde_json::to_string_pretty(&theme_to_cache).unwrap(),
        )
        .unwrap();

        let result = orchestrator.load_theme(theme, false, false, &ctx.state);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "Vesper");
    }

    #[test]
    fn should_ignore_cache_when_force_is_true() {
        let (_temp, mut ctx) = create_test_context();
        ctx.state.manager = PluginManager::Lazy;
        let orchestrator = ThemeOrchestrator::new(&ctx.paths, &ctx.log);

        let theme = "habamax";
        let cache_path = ctx.paths.themes.join(format!("{}.json", theme));
        let old_palette_json = r##"{
            "name": "Habamax",
            "colors": {
                "bg": "#000000",
                "fg": "",
                "caret": "",
                "line_hl": "",
                "sel": "",
                "gutter_fg": "",
                "comment": "",
                "variable": "",
                "constant": "",
                "number": "",
                "string": "",
                "keyword": "",
                "operator": "",
                "func": "",
                "type_name": "",
                "tag": "",
                "attribute": "",
                "white": "",
                "ansi": []
            }
        }"##;

        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&cache_path, old_palette_json).unwrap();

        let cached_res = orchestrator
            .load_theme(theme, false, false, &ctx.state)
            .unwrap();
        assert_eq!(cached_res.colors.bg, "#000000");

        let forced_res = orchestrator.load_theme(theme, true, false, &ctx.state);
        match forced_res {
            Ok(theme_obj) => {
                assert_ne!(theme_obj.colors.bg, "#000000", "Should bypass cache");
            }
            Err(e) => {
                eprintln!(
                    "Bypassed cache successfully! Neovim missing/failed on CI: {}",
                    e
                );
            }
        }
    }

    #[test]
    fn should_only_save_to_cache_when_save_flag_is_true() {
        let (_temp, ctx) = create_test_context();
        let orchestrator = ThemeOrchestrator::new(&ctx.paths, &ctx.log);
        let theme = "habamax";
        let cache_path = ctx.paths.themes.join(format!("{}.json", theme));

        if cache_path.exists() {
            fs::remove_file(&cache_path).unwrap();
        }

        let _ = orchestrator.load_theme(theme, false, false, &ctx.state);
        assert!(
            !cache_path.exists(),
            "Should not create file when save = false"
        );
    }
}
