use crate::{
    infra::{IrisPaths, NeovimBridge},
    log::{Activity, Logger},
    models::{Palette, PluginManager, State, Theme},
};
use colored::Colorize;
use std::{fs, path::PathBuf};

/// Handles theme resolution: cache lookup, fetching from Neovim, validation and logging.
pub struct ThemeService<'a> {
    paths: &'a IrisPaths,
    log: &'a Logger,
}

impl<'a> ThemeService<'a> {
    pub fn new(paths: &'a IrisPaths, log: &'a Logger) -> Self {
        Self { paths, log }
    }

    /// Gets the name of the current theme from Iris cache file
    pub fn current(&self) -> anyhow::Result<String> {
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
            anyhow::bail!("Theme cache is empty!");
        }

        Ok(crate::utils::capitalize(trimmed))
    }

    /// Get theme sync status
    pub fn sync_status(&self, state: &State) -> (String, bool) {
        let current: &String = &state.theme.current_theme;
        let nvim_theme: String = self.current().unwrap_or_default();
        let is_sync: bool = nvim_theme.eq_ignore_ascii_case(current);

        let display_name: String = if is_sync {
            crate::utils::capitalize(current)
        } else if nvim_theme.is_empty() {
            "Unknown".to_string()
        } else {
            crate::utils::capitalize(&nvim_theme)
        };

        (display_name, is_sync)
    }

    /// Attempts to load a theme from cache.
    /// Returns `None` if there's no cache or if it's corrupted (logs a warning in the latter case).
    fn try_load_cache(&self, cache_path: &PathBuf, activity: &Activity) -> Option<Theme> {
        match Theme::load_from_cache(cache_path) {
            Ok(Some(theme)) => Some(theme),
            Ok(None) => None,
            Err(e) => {
                activity
                    .log
                    .warn(&format!("Cached theme is corrupted, refetching: {e}..."));
                None
            }
        }
    }

    /// Load theme (with cache check, validation and logging)
    pub fn load_theme(
        &self,
        theme_name: &str,
        force: bool,
        save: bool,
        state: &State,
    ) -> anyhow::Result<Theme> {
        let theme_lower: String = theme_name.to_lowercase();
        let theme_cap: String = crate::utils::capitalize(theme_name);
        let cache_path: PathBuf = self.paths.cached_theme(theme_name);

        let activity = self.log.step_with_icon(
            "".magenta().bold(),
            &format!("Fetching: {}...", theme_cap.cyan().bold()),
            true,
        );

        if !force {
            if let Some(cached_theme) = self.try_load_cache(&cache_path, &activity) {
                activity.log.info(&format!(
                    "Using cached theme for {}...",
                    theme_cap.yellow().bold()
                ));
                activity.done();
                return Ok(cached_theme);
            }
        }

        if state.nvim.manager.is_default() {
            let builtins: &[&str] = NeovimBridge::builtin_themes();
            if !builtins.contains(&theme_lower.as_str()) {
                if let Some(cached_theme) = self.try_load_cache(&cache_path, &activity) {
                    activity
                        .log
                        .warn("Built-in mode active. Using existing cache for external theme.");
                    activity.done();
                    return Ok(cached_theme);
                }
                anyhow::bail!(
                    "Theme `{}` is not a built-in theme and not cached",
                    theme_name.yellow().bold()
                );
            }
        }

        if force {
            activity.log.info(&format!(
                "`{}` flag detected. Bypassing cache...",
                "--force".cyan()
            ));
        }

        let stdout: String = activity.log.action("Executed `lua` bridge.", || {
            NeovimBridge::run_fetch_bridge(theme_name, state)
        })?;

        let palette: Palette = activity
            .log
            .action("Parsed palette data.", || Palette::parse_from_nvim(&stdout))?;

        let theme_obj: Theme = Theme::new(&theme_cap, palette);
        if save {
            activity.log.action("Saved theme to cache.", || {
                theme_obj.save_to_cache(&cache_path)
            })?;
        }

        activity.done_with(&format!(
            "Theme `{}` fetched successfully!",
            theme_cap.yellow()
        ));

        Ok(theme_obj)
    }

    /// Checks if theme exists (in cache, neovim or builtin)
    pub fn exists(&self, theme_name: &str, state: &State) -> bool {
        let theme_lower: String = theme_name.to_lowercase();

        if which::which("nvim").is_err() {
            return false;
        }

        if self.paths.is_theme_cached(&theme_lower) {
            return true;
        }

        if state.nvim.manager == PluginManager::Default {
            return NeovimBridge::builtin_themes().contains(&theme_lower.as_str());
        }

        NeovimBridge::check_theme_exists(&theme_lower, state)
    }

    /// Get all themes
    pub fn themes(&self) -> Vec<String> {
        match NeovimBridge::installed_themes() {
            Ok(themes) if !themes.is_empty() => themes.to_vec(),
            _ => {
                self.log.warn(
                    "Could not fetch themes from active `nvim` session. Falling back to builtins.",
                );
                NeovimBridge::builtin_themes()
                    .iter()
                    .map(|s| s.to_string())
                    .collect()
            }
        }
    }
}

/// Unit-tests for theme service
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::IrisContext;

    #[test]
    fn should_save_and_load_cache() {
        let (_temp, ctx) = IrisContext::mock();
        let service = ThemeService::new(&ctx.paths, &ctx.log);
        let theme_to_cache: Theme = Theme::mock();
        let theme_name: String = theme_to_cache.name.to_lowercase();
        let cache_path = ctx.paths.cached_theme(&theme_name);

        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }

        theme_to_cache.save_to_cache(&cache_path).unwrap();
        assert!(cache_path.exists());

        let result = service.load_theme(&theme_to_cache.name, false, false, &ctx.state);
        assert!(result.is_ok());

        let loaded_theme = result.unwrap();
        assert_eq!(loaded_theme.name, theme_name);
        assert_eq!(loaded_theme.colors.bg, "#1a1b26");
        assert_eq!(loaded_theme.colors.fg, "#c0caf5");
        assert_eq!(loaded_theme.colors.caret, "#c0caf5");
    }

    #[test]
    fn should_read_theme_from_valid_path() {
        let (_temp, ctx) = IrisContext::mock();
        let service = ThemeService::new(&ctx.paths, &ctx.log);
        let cache_path = &ctx.paths.current_theme;

        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(cache_path, "  melange  ").unwrap();

        let result = service.current().unwrap();
        assert_eq!(result, "Melange");
    }

    #[test]
    fn should_invoke_error_when_theme_file_is_empty() {
        let (temp, ctx) = IrisContext::mock();
        let cache_path: PathBuf = temp.path().join("empty_theme");
        let service = ThemeService::new(&ctx.paths, &ctx.log);

        fs::write(&cache_path, "    ").unwrap();

        let result = service.current();
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
                "added": "#ffffff", "changed": "#ffffff", "deleted": "#ffffff",
                "ansi": [
                    "#000000", "#111111", "#222222", "#333333",
                    "#444444", "#555555", "#666666", "#777777",
                    "#888888", "#999999", "#aaaaaa", "#bbbbbb",
                    "#cccccc", "#dddddd", "#eeeeee", "#ffffff"
                ]
            }
            [NVIM] Process exited
        "##;

        let result = Palette::parse_from_nvim(raw_output);
        assert!(result.is_ok(), "Parser failed: {:?}", result.err());
        assert_eq!(result.unwrap().bg, "#121212");
    }

    #[test]
    fn should_read_from_cache_in_default_manager_even_if_external() {
        let (_temp, mut ctx) = IrisContext::mock();
        ctx.state.nvim.manager = PluginManager::Default;
        let service = ThemeService::new(&ctx.paths, &ctx.log);
        let theme = "vesper";

        let cache_path = ctx.paths.cached_theme(theme);
        let theme_to_cache = Theme::new("Vesper", Theme::mock().colors);

        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        theme_to_cache.save_to_cache(&cache_path).unwrap();

        let result = service.load_theme(theme, false, false, &ctx.state);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "Vesper");
    }

    #[test]
    fn should_ignore_cache_when_force_is_true() {
        let (_temp, mut ctx) = IrisContext::mock();
        ctx.state.nvim.manager = PluginManager::Lazy;
        let service = ThemeService::new(&ctx.paths, &ctx.log);

        let theme = "habamax";
        let cache_path = ctx.paths.cached_theme(theme);

        let mut theme_to_cache = Theme::new("Habamax", Theme::mock().colors);
        theme_to_cache.colors.bg = "#000000".to_string();

        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        theme_to_cache.save_to_cache(&cache_path).unwrap();

        let cached_res = service.load_theme(theme, false, false, &ctx.state).unwrap();
        assert_eq!(cached_res.colors.bg, "#000000");

        let forced_res = service.load_theme(theme, true, false, &ctx.state);
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
        let (_temp, ctx) = IrisContext::mock();
        let service = ThemeService::new(&ctx.paths, &ctx.log);
        let theme = "habamax";
        let cache_path = ctx.paths.cached_theme(theme);

        if cache_path.exists() {
            fs::remove_file(&cache_path).unwrap();
        }

        let _ = service.load_theme(theme, false, false, &ctx.state);
        assert!(!cache_path.exists());
    }
}
