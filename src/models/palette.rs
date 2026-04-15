use crate::{ui::Logger, utils};
use anyhow::{Context, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
};

/// Theme palette that is being retreived from nvim
#[derive(Debug, Deserialize, Serialize)]
pub struct Palette {
    #[serde(default)]
    pub name: String,

    pub bg: String,
    pub fg: String,
    pub caret: String,
    pub line_hl: String,
    pub sel: String,
    pub gutter_fg: String,
    pub comment: String,
    pub variable: String,
    pub constant: String,
    pub number: String,
    pub string: String,
    pub keyword: String,
    pub operator: String,
    pub func: String,
    pub type_name: String,
    pub tag: String,
    pub attribute: String,
    pub white: String,
    pub ansi: Vec<String>,
}

impl Palette {
    /// Get currently installed theme name
    /// Uses logger for warnings and errors output
    pub fn current(log: &Logger) -> Result<String> {
        let home: PathBuf = dirs::home_dir().context("Home dir not found")?;
        let path: PathBuf = home.join(".cache/iris/current_theme");

        match Self::read_theme_from_path(&path) {
            Ok(theme) => Ok(theme),
            Err(_) => {
                log.error("No active theme found in cache.", 1);
                anyhow::bail!(
                    "No active theme detected.\n {} Make sure to switch theme in Neovim or pass the name manually: `iris switch <name>`",
                    "Tip:".yellow()
                );
            }
        }
    }

    /// Fetch palette from nvim using lua script
    pub fn fetch(theme: &str, log: &Logger) -> Result<Self> {
        let theme_lower: String = theme.to_lowercase();
        let cache_dir: PathBuf = dirs::home_dir()
            .context("Home dir not found")?
            .join(".cache/iris/palettes");

        let cache_path: PathBuf = cache_dir.join(format!("{}.json", theme_lower));

        if cache_path.exists() {
            if let Ok(content) = fs::read_to_string(&cache_path) {
                if let Ok(cached) = serde_json::from_str::<Self>(&content) {
                    log.info(&format!(
                        "Using cached palette for {}...",
                        theme.yellow().bold()
                    ));
                    return Ok(cached);
                }
            }
        }

        log.info("Cache miss. Loading Neovim runtime and lazy.nvim plugins...");
        let args: Vec<String> = Self::build_fetch_args(theme);

        log.info("Executing Lua bridge in headless mode...");
        let output: Output = Command::new("nvim").args(&args).output()?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            log.error(&format!("Neovim error: {}", error_msg.trim()), 2);
            anyhow::bail!("Neovim error: {}", error_msg.red());
        }

        log.info("Parsing palette data...");
        let stdout = String::from_utf8_lossy(&output.stdout);

        let mut palette: Palette = Self::parse_nvim_json(&stdout)?;
        palette.name = theme.to_string();

        if let Err(e) = Self::save_to_cache(&cache_path, &palette) {
            log.warn(&format!("Failed to cache palette: {}", e), 1);
        }

        Ok(palette)
    }

    /// Checks whether this theme exists in nvim colorscheme
    pub fn exists(theme: &str, log: &Logger) -> bool {
        let theme_lower: String = theme.to_lowercase();
        let home: PathBuf = dirs::home_dir().expect("Home dir not found");
        let cache_path: PathBuf = home
            .join(".cache/iris/palettes")
            .join(format!("{}.json", theme_lower));

        if cache_path.exists() {
            println!(
                "   {} found in cache!",
                utils::capitalize(&theme_lower).yellow().bold()
            );
            return true;
        }

        if which::which("nvim").is_err() {
            log.warn("Neovim not found. Skipping theme verification.", 0);
            return false;
        }

        let mut check_task = log.step(
            &format!("Checking theme availability: {}", theme.yellow().bold()),
            1,
        );

        let args: Vec<String> = Self::build_exists_args(theme);
        let success: bool = Command::new("nvim")
            .args(&args)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if success {
            check_task.done(true);
        } else {
            log.error(&format!("Theme {} not found in Neovim", theme.red()), 2);
        }

        success
    }

    /// Helper function to save palette to cache
    pub fn save_to_cache(path: &PathBuf, palette: &Palette) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create cache directory: {:?}", parent))?;
        }

        let json: String =
            serde_json::to_string_pretty(palette).context("Failed to serialize palette to JSON")?;

        fs::write(path, json)
            .with_context(|| format!("Failed to write palette cache to {:?}", path))?;
        Ok(())
    }

    /// Helper function to read theme from given path (easy to test)
    fn read_theme_from_path(path: &PathBuf) -> Result<String> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read theme cache at {:?}", path))?;

        let trimmed = content.trim();
        if trimmed.is_empty() {
            anyhow::bail!("Theme cache is empty");
        }

        let mut chars = trimmed.chars();
        Ok(match chars.next() {
            None => String::new(),
            Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
        })
    }

    /// Helper function to build command args for nvim (palette fetch command)
    fn build_fetch_args(theme: &str) -> Vec<String> {
        let lua_script: &str = Self::fetch_lua_script();
        vec![
            "--headless".to_string(),
            "-u".to_string(),
            "NONE".to_string(),
            "-c".to_string(),
            "lua vim.opt.rtp:append(vim.fn.stdpath('data') .. '/lazy/*')".to_string(),
            "-c".to_string(),
            format!("colorscheme {}", theme.to_lowercase()),
            "-c".to_string(),
            format!("lua {}", lua_script),
            "-c".to_string(),
            "qa!".to_string(),
        ]
    }

    /// Helper function to build command args for nvim (exists palette command)
    fn build_exists_args(theme: &str) -> Vec<String> {
        let init_plugins: &str = "lua vim.opt.rtp:append(vim.fn.stdpath('data') .. '/lazy/*')";
        let check_cmd: String = format!(
            "try | colorscheme {} | qa! | catch | cquit 1 | endtry",
            theme.to_lowercase()
        );
        vec![
            "--headless".to_string(),
            "-u".to_string(),
            "NONE".to_string(),
            "-c".to_string(),
            init_plugins.to_string(),
            "-c".to_string(),
            check_cmd,
        ]
    }

    /// Helper function to parse returned from nvim json file with palette info
    fn parse_nvim_json(stdout: &str) -> Result<Self> {
        let json_start: usize = stdout.find('{').context("Nvim did not return JSON")?;
        let json_end: usize = stdout.rfind('}').context("JSON is malformed")? + 1;
        serde_json::from_str(&stdout[json_start..json_end]).context("Failed to parse palette JSON")
    }

    /// Lua script to fetch palette from nvim
    fn fetch_lua_script() -> &'static str {
        r##"
        local function g(name, attr)
            local max_depth = 10
            local current = name

            for _ = 1, max_depth do
                local hl = vim.api.nvim_get_hl(0, { name = current, link = false })
                local color = hl[attr or 'fg'] or hl.bg

                if color then
                    return string.format('#%06x', color)
                end

                local linked = vim.api.nvim_get_hl(0, { name = current, link = true })
                if not linked.link then
                    break
                end

                current = linked.link
            end

            return '#cccccc'
        end

        local function first(attr, names)
            for _, name in ipairs(names) do
                local c = g(name, attr)
                if c ~= '#cccccc' then
                    return c
                end
            end
            return '#cccccc'
        end

        local ansi = {}
        for i = 0, 15 do
            table.insert(ansi, vim.g['terminal_color_' .. i] or '#000000')
        end

        local res = {
            bg         = g('Normal', 'bg'),
            fg         = g('Normal', 'fg'),
            caret      = first('bg', { 'Cursor', 'TermCursor' }),
            line_hl    = first('bg', { 'CursorLine', 'CursorLineBg' }),
            sel        = first('bg', { 'Visual', 'Selection' }),
            gutter_fg  = first('fg', { 'LineNr', 'SignColumn' }),
            comment    = first('fg', { 'Comment', '@comment' }),
            variable   = first('fg', { '@variable', 'Identifier' }),
            constant   = first('fg', { 'Constant', '@constant' }),
            number     = first('fg', { 'Number', '@number', 'Constant' }),
            string     = first('fg', { 'String', '@string' }),
            keyword    = first('fg', { 'Keyword', '@keyword', 'Statement' }),
            operator   = first('fg', { 'Operator', '@operator' }),
            func       = first('fg', { 'Function', '@function' }),
            type_name  = first('fg', { 'Type', '@type' }),
            tag        = first('fg', { 'Tag', '@tag' }),
            attribute  = first('fg', { '@attribute', '@property' }),
            added      = first('fg', { 'DiffAdd', 'GitSignsAdd', '@diff.plus' }),
            deleted    = first('fg', { 'DiffDelete', 'GitSignsDelete', '@diff.minus' }),
            changed    = first('fg', { 'DiffChange', 'GitSignsChange', '@diff.delta' }),
            white      = vim.g.terminal_color_15 or '#ffffff',
            ansi       = ansi,
        }

        io.write(vim.fn.json_encode(res))
        "##
    }
}

/// Unit-tests for palette operation
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempdir::TempDir;

    #[test]
    fn should_parse_palette_json() {
        let json = r##"{
            "bg": "#1e1e2e",
            "fg": "#cdd6f4",
            "caret": "#f5e0dc",
            "line_hl": "#313244",
            "sel": "#45475a",
            "gutter_fg": "#45475a",
            "comment": "#6c7086",
            "variable": "#f38ba8",
            "constant": "#fab387",
            "number": "#fab387",
            "string": "#a6e3a1",
            "keyword": "#cba6f7",
            "operator": "#89dceb",
            "func": "#89b4fa",
            "type_name": "#f9e2af",
            "tag": "#f38ba8",
            "attribute": "#f9e2af",
            "white": "#ffffff",
            "ansi": ["#1e1e2e", "#f38ba8", "#a6e3a1", "#f9e2af", "#89b4fa", "#cba6f7", "#89dceb", "#bac2de", "#585b70", "#f38ba8", "#a6e3a1", "#f9e2af", "#89b4fa", "#cba6f7", "#89dceb", "#a6adc8"]
            }"##;

        let palette: Palette = serde_json::from_str(json).unwrap();
        assert_eq!(palette.bg, "#1e1e2e");
        assert_eq!(palette.ansi.len(), 16);
    }

    #[test]
    fn should_save_and_load_cache() {
        let temp_dir: TempDir = TempDir::new("iris_cache").unwrap();
        let palette_dir: PathBuf = temp_dir.path().join("core/palettes");
        let cache_path: PathBuf = palette_dir.join("catppuccin.json");

        let palette = Palette {
            name: "Catppuccin".to_string(),
            bg: "#1e1e2e".to_string(),
            fg: "#cdd6f4".to_string(),
            ..serde_json::from_str(
                r##"{
                    "bg":"#1e1e2e",
                    "fg":"#cdd6f4",
                    "caret":"",
                    "line_hl":""
                    ,"sel":"",
                    "gutter_fg":"",
                    "comment":"",
                    "variable":"",
                    "constant":"",
                    "number":"",
                    "string":"",
                    "keyword":"",
                    "operator":"",
                    "func":"",
                    "type_name":"",
                    "tag":"",
                    "attribute":"",
                    "white":"",
                    "ansi":[]}"##,
            )
            .unwrap()
        };

        Palette::save_to_cache(&cache_path, &palette).expect("Should save cache");
        assert!(cache_path.exists());

        let content = fs::read_to_string(&cache_path).unwrap();
        let loaded: Palette = serde_json::from_str(&content).unwrap();
        assert_eq!(loaded.name, "Catppuccin");
    }

    #[test]
    fn should_read_theme_from_valid_path() {
        let temp_dir: TempDir = TempDir::new("iris_cache").unwrap();
        let cache_path: PathBuf = temp_dir.path().join("current_theme");

        fs::write(&cache_path, "  melange  ").unwrap();

        let result: String = Palette::read_theme_from_path(&cache_path).unwrap();
        assert_eq!(result, "Melange");
    }

    #[test]
    fn should_invoke_error_when_theme_file_is_empty() {
        let temp_dir: TempDir = TempDir::new("iris_cache").unwrap();
        let cache_path: PathBuf = temp_dir.path().join("empty_theme");

        fs::write(&cache_path, "    ").unwrap();

        let result = Palette::read_theme_from_path(&cache_path);
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
                "ansi": []
            }
            [NVIM] Process exited
        "##;

        let result = Palette::parse_nvim_json(raw_output);

        assert!(result.is_ok(), "Parse failed: {:?}", result.err());
        assert_eq!(result.unwrap().bg, "#121212");
    }

    #[test]
    fn should_test_build_fetch_args_case() {
        let theme = "Tokyonight";
        let args = Palette::build_fetch_args(theme);

        let has_lowercase = args.iter().any(|a| a.contains("colorscheme tokyonight"));
        assert!(
            has_lowercase,
            "Arguments should contain lowercase colorscheme command"
        );
    }

    #[test]
    fn should_test_build_exists_args_case() {
        let args = Palette::build_exists_args("gruvbox");

        assert!(args.contains(&"--headless".to_string()));
        assert!(args.contains(&"NONE".to_string()));
        assert!(args.iter().any(|a| a.contains("cquit 1")));
    }
}
