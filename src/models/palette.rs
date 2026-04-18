use crate::core::IrisContext;
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
    pub fn current() -> Result<String> {
        let home: PathBuf = dirs::home_dir().context("Failed to determine home directory")?;
        let path: PathBuf = home.join(".cache/iris/core/current_theme");

        Self::read_theme_from_path(&path).map_err(|_| {
            anyhow::anyhow!(format!(
                "No active theme detected.\n\
                 {}: Make sure to switch theme in Neovim or pass the name manually: `{}`",
                "Tip".bold().cyan(),
                "iris switch <name>".italic().cyan()
            ))
        })
    }

    /// Fetch palette from nvim using lua script
    pub fn fetch(theme: &str, ctx: &IrisContext) -> Result<Self> {
        let theme_lower: String = theme.to_lowercase();
        let cache_dir = dirs::home_dir()
            .context("Failed to determine home directory")?
            .join(".cache/iris/core/palettes");

        let cache_path: PathBuf = cache_dir.join(format!("{}.json", theme_lower));

        if cache_path.exists() {
            if let Ok(content) = fs::read_to_string(&cache_path) {
                if let Ok(cached) = serde_json::from_str::<Self>(&content) {
                    ctx.log.info(&format!(
                        "Using cached palette for {}...",
                        theme.yellow().bold()
                    ));
                    return Ok(cached);
                }
            }
        }

        ctx.log
            .info("Cache miss. Loading Neovim runtime and plugins...");
        let args: Vec<String> = Self::build_fetch_args(theme, &ctx);

        ctx.log.info("Executing Lua bridge in headless mode...");
        let output: Output = Command::new("nvim")
            .args(&args)
            .output()
            .context("Failed to execute 'nvim' command. Is Neovim installed?")?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Neovim failed to export palette: {}", error_msg.trim());
        }

        ctx.log.info("Parsing palette data...");
        let stdout = String::from_utf8_lossy(&output.stdout);

        let mut palette: Palette = Self::parse_nvim_json(&stdout)?;
        palette.name = theme.to_string();

        if let Err(e) = Self::save_to_cache(&cache_path, &palette) {
            ctx.log.warn(&format!("Failed to cache palette: {}", e), 1);
        }

        Ok(palette)
    }

    /// Checks whether this theme exists in nvim colorscheme
    pub fn exists(theme: &str, ctx: &IrisContext) -> bool {
        let theme_lower: String = theme.to_lowercase();
        let cache_dir = dirs::home_dir().map(|h| h.join(".cache/iris/core/palettes"));

        if let Some(path) = cache_dir {
            if path.join(format!("{}.json", theme_lower)).exists() {
                return true;
            }
        }

        if which::which("nvim").is_err() {
            return false;
        }

        let args: Vec<String> = Self::build_exists_args(theme, ctx);
        let output = Command::new("nvim").args(&args).output();

        match output {
            Ok(o) if o.status.success() => true,
            _ => false,
        }
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

    /// Helper function to build base nvim arguments for fetch and exists
    fn build_base_args(ctx: &IrisContext) -> Vec<String> {
        let mut args: Vec<String> = vec![
            "--headless".to_string(),
            "-u".to_string(),
            "NONE".to_string(),
        ];

        if let Some(rtp_cmd) = ctx.state.get_rtp_command() {
            args.push("-c".into());
            args.push(rtp_cmd);
        }
        args
    }

    /// Helper function to build command args for nvim (palette fetch command)
    fn build_fetch_args(theme: &str, ctx: &IrisContext) -> Vec<String> {
        let mut args: Vec<String> = Self::build_base_args(ctx);
        args.extend([
            "-c".into(),
            format!("colorscheme {}", theme.to_lowercase()),
            "-c".into(),
            format!("lua {}", Self::fetch_lua_script()),
            "-c".into(),
            "qa!".into(),
        ]);
        args
    }

    /// Helper function to build command args for nvim (exists palette command)
    fn build_exists_args(theme: &str, ctx: &IrisContext) -> Vec<String> {
        let mut args: Vec<String> = Self::build_base_args(ctx);
        args.extend([
            "-c".into(),
            format!(
                "try | colorscheme {} | qa! | catch | cquit 1 | endtry",
                theme.to_lowercase()
            ),
        ]);
        args
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
            local max_depth = 15
            local current = name
            local visited = {}

            for _ = 1, max_depth do
                if visited[current] then break end
                visited[current] = true

                local hl = vim.api.nvim_get_hl(0, { name = current, link = false })

                local color = nil
                if attr == 'bg' then
                    color = hl.bg
                else
                    color = hl.fg
                end

                if color ~= nil then
                    return string.format('#%06x', color)
                end

                local linked = vim.api.nvim_get_hl(0, { name = current, link = true })
                if not linked.link or linked.link == current then
                    break
                end
                current = linked.link
            end

            return nil
        end

        local function first(attr, names)
            for _, name in ipairs(names) do
                local c = g(name, attr)
                if c then return c end
            end
            return nil
        end

        local function chain(attr, ...)
            for _, names in ipairs({...}) do
                local c = first(attr, names)
                if c then return c end
            end
            return '#cccccc'
        end

        local ansi = {}
        for i = 0, 15 do
            local color = vim.g['terminal_color_' .. i]
            if type(color) == 'string' then
                table.insert(ansi, color)
            elseif type(color) == 'number' then
                table.insert(ansi, string.format('#%06x', color))
            else
                table.insert(ansi, i < 8 and '#000000' or '#ffffff')
            end
        end

        local fg = g('Normal', 'fg') or '#cccccc'
        local bg = g('Normal', 'bg') or '#101010'

        local res = {
            bg        = bg,
            fg        = fg,

            caret     = chain('bg',
                { 'Cursor', 'TermCursor' },
                { 'CursorLine' }
            ),

            line_hl   = chain('bg',
                { 'CursorLine', 'CursorLineBg' },
                { 'ColorColumn' }
            ),

            sel       = chain('bg',
                { 'Visual', 'Selection', 'PmenuSel' }
            ),

            gutter_fg = chain('fg',
                { 'LineNr', 'SignColumn', 'FoldColumn' },
                { 'Comment' }
            ),

            comment   = chain('fg',
                { 'Comment', '@comment', '@comment.line', '@comment.block' }
            ),

            variable  = chain('fg',
                { '@variable', '@variable.member', '@variable.parameter' },
                { 'Identifier' },
                { 'Normal' }
            ),

            constant  = chain('fg',
                { '@constant', '@constant.builtin', '@constant.macro' },
                { 'Constant', 'Special' }
            ),

            number    = chain('fg',
                { '@number', '@number.float', '@number.integer' },
                { 'Number', 'Float' },
                { 'Constant' }
            ),

            string    = chain('fg',
                { '@string', '@string.special', '@string.escape' },
                { 'String', 'Character' }
            ),

            keyword   = chain('fg',
                { '@keyword', '@keyword.function', '@keyword.operator', '@keyword.import', '@keyword.return' },
                { 'Keyword', 'Statement', 'Conditional', 'Repeat' }
            ),

            operator  = chain('fg',
                { '@operator', '@keyword.operator' },
                { 'Operator' },
                { 'Normal' }
            ),

            func      = chain('fg',
                { '@function', '@function.call', '@function.builtin', '@function.method', '@function.method.call' },
                { 'Function' }
            ),

            type_name = chain('fg',
                { '@type', '@type.builtin', '@type.definition' },
                { 'Type', 'Typedef' }
            ),

            tag       = chain('fg',
                { '@tag', '@tag.builtin' },
                { 'Tag', 'Special' }
            ),

            attribute = chain('fg',
                { '@attribute', '@property', '@tag.attribute' },
                { 'Identifier' }
            ),

            added     = chain('fg',
                { 'DiffAdd', 'GitSignsAdd', '@diff.plus', 'Added' }
            ),

            deleted   = chain('fg',
                { 'DiffDelete', 'GitSignsDelete', '@diff.minus', 'Removed' }
            ),

            changed   = chain('fg',
                { 'DiffChange', 'GitSignsChange', '@diff.delta', 'Changed' }
            ),

            white     = (vim.g.terminal_color_15 ~= nil
                and (type(vim.g.terminal_color_15) == 'string'
                    and vim.g.terminal_color_15
                    or string.format('#%06x', vim.g.terminal_color_15))
                or '#ffffff'),

            ansi      = ansi,
        }

        io.write(vim.fn.json_encode(res))
        "##
    }

    #[cfg(test)]
    /// Function to create palette mock
    pub fn mock() -> Self {
        Self {
            name: "test-theme".into(),
            bg: "#1a1b26".into(),
            fg: "#c0caf5".into(),
            caret: "#c0caf5".into(),
            line_hl: "#292e42".into(),
            sel: "#334455".into(),
            gutter_fg: "#3b4261".into(),
            comment: "#565f89".into(),
            variable: "#bb9af7".into(),
            constant: "#ff9e64".into(),
            number: "#ff9e64".into(),
            string: "#9ece6a".into(),
            keyword: "#7aa2f7".into(),
            operator: "#89ddff".into(),
            func: "#7ad6ff".into(),
            type_name: "#2ac3de".into(),
            tag: "#f7768e".into(),
            attribute: "#e0af68".into(),
            white: "#ffffff".into(),
            ansi: (0..16).map(|_| "#ffffff".to_string()).collect(),
        }
    }
}

/// Unit-tests for palette operation
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{core::tests::create_test_context, models::NvimStrategy};
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
        let (_temp, ctx) = create_test_context();
        let theme = "Tokyonight";
        let args = Palette::build_fetch_args(theme, &ctx);

        assert!(args.contains(&"--headless".to_string()));
        assert!(args.contains(&"NONE".to_string()));

        let has_lowercase = args.iter().any(|a| a == "colorscheme tokyonight");
        assert!(
            has_lowercase,
            "Arguments should contain 'colorscheme tokyonight'"
        );
    }

    #[test]
    fn should_test_build_exists_args_case() {
        let (_temp, mut ctx) = create_test_context();
        ctx.state.nvim = NvimStrategy::Lazy;
        let args = Palette::build_exists_args("gruvbox", &ctx);

        assert!(args.contains(&"--headless".to_string()));
        assert!(args.contains(&"NONE".to_string()));
        assert!(args.iter().any(|a| a.contains("cquit 1")));

        let has_rtp = args.iter().any(|a| a.contains("vim.opt.rtp:append"));
        assert!(has_rtp, "Lazy strategy must include RTP setup in arguments");
    }

    #[test]
    fn should_test_build_args_without_rtp_for_default_strategy() {
        let (_temp, mut ctx) = create_test_context();
        ctx.state.nvim = NvimStrategy::Default;
        let args = Palette::build_exists_args("default_theme", &ctx);

        let has_rtp = args.iter().any(|a| a.contains("vim.opt.rtp:append"));
        assert!(!has_rtp, "Default strategy should NOT include RTP setup");
    }
}
