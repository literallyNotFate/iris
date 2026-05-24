use crate::{
    core::{Client, IrisPaths},
    log::Reporter,
    models::{PluginManager, State},
    utils::{self, CustomColor},
};
use anyhow::{Context, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf, process::Command};

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
                "iris switch <name>".bold().cyan()
            ))
        })
    }

    /// Fetch palette from nvim using lua script
    pub fn fetch(
        theme: &str,
        force: bool,
        save: bool,
        paths: &IrisPaths,
        state: &State,
        log: &Reporter,
    ) -> Result<Self> {
        let theme_lower = theme.to_lowercase();
        let cache_path = paths.palettes.join(format!("{}.json", theme_lower));

        let main_task = log.step_with_icon(
            "".magenta().bold(),
            &format!("Fetching palette: {}", theme.cyan().bold()),
            true,
        );

        if !force && cache_path.exists() {
            if let Ok(content) = fs::read_to_string(&cache_path) {
                if let Ok(cached) = serde_json::from_str::<Self>(&content) {
                    main_task.log.info(&format!(
                        "Using cached palette for {}...",
                        theme.yellow().bold()
                    ));
                    main_task.done();
                    return Ok(cached);
                }
            }
        }

        if matches!(state.manager, PluginManager::Default) {
            let builtins = Client::get_builtin_themes();
            if !builtins.contains(&theme_lower) {
                if cache_path.exists() {
                    if let Ok(content) = fs::read_to_string(&cache_path) {
                        if let Ok(cached) = serde_json::from_str::<Self>(&content) {
                            main_task.log.warn(
                                "Built-in mode active. Using existing cache for external theme.",
                            );
                            main_task.done();
                            return Ok(cached);
                        }
                    }
                }

                anyhow::bail!(
                    "Theme `{}` is not a built-in theme and not cached",
                    theme.yellow().bold()
                );
            }
        }

        if force {
            main_task.log.info(&format!(
                "`{}` flag detected. Bypassing cache...",
                "--force".cyan()
            ));
        }

        let output = main_task.log.action("Executed Lua bridge", || {
            let args = Self::build_fetch_args(theme, state);
            Command::new("nvim")
                .args(&args)
                .output()
                .context("Failed to execute `nvim`")
        })?;
        println!();

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Neovim failed to export palette: {}", error_msg.trim());
        }

        let mut palette: Palette = main_task.log.action("Parsed palette data", || {
            let stdout = String::from_utf8_lossy(&output.stdout);
            Self::parse_nvim_json(&stdout)
        })?;

        palette.name = theme.to_string();
        println!();

        if save {
            main_task.log.action("Saved palette to cache", || {
                Self::save_to_cache(&cache_path, &palette)
            })?;
            println!();
        }

        main_task.done_with(&format!(
            "Palette `{}` fetched successfully!",
            utils::capitalize(&palette.name).yellow()
        ));
        Ok(palette)
    }

    /// Checks whether this theme exists in nvim colorscheme
    pub fn exists(theme: &str, paths: &IrisPaths, state: &State) -> bool {
        let theme_lower: String = theme.to_lowercase();
        if paths
            .palettes
            .join(format!("{}.json", theme_lower))
            .exists()
        {
            return true;
        }

        if matches!(state.manager, PluginManager::Default) {
            return Client::get_builtin_themes().contains(&theme_lower);
        }

        if which::which("nvim").is_err() {
            return false;
        }

        let args: Vec<String> = Self::build_exists_args(&theme_lower, state);
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
        let content: String = fs::read_to_string(path)
            .with_context(|| format!("Failed to read theme cache at {:?}", path))?;

        let trimmed: &str = content.trim();
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
    fn build_base_args(state: &State) -> Vec<String> {
        let mut args: Vec<String> = vec!["--headless".to_string()];

        match state.manager {
            PluginManager::Default => {
                args.push("-u".into());
                args.push("NONE".into());
            }
            _ => {
                args.push("-u".into());
                args.push("NONE".into());

                if let Some(rtp_cmd) = state.get_rtp_command() {
                    args.push("-c".into());
                    args.push(rtp_cmd);
                }
            }
        }
        args
    }

    /// Helper function to build command args for nvim (palette fetch command)
    fn build_fetch_args(theme: &str, state: &State) -> Vec<String> {
        let mut args: Vec<String> = Self::build_base_args(state);
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
    fn build_exists_args(theme: &str, state: &State) -> Vec<String> {
        let mut args: Vec<String> = Self::build_base_args(state);
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

        local fg = g('Normal', 'fg') or '#cccccc'
        local bg = g('Normal', 'bg') or '#1c1c1c'

        local function resolve_ansi()
            local result = {}
            local has_any = false

            for i = 0, 15 do
                if vim.g['terminal_color_' .. i] ~= nil then
                    has_any = true
                    break
                end
            end

            if has_any then
                for i = 0, 15 do
                    local color = vim.g['terminal_color_' .. i]
                    if type(color) == 'string' then
                        table.insert(result, color)
                    elseif type(color) == 'number' then
                        table.insert(result, string.format('#%06x', color))
                    else
                        table.insert(result, i < 8 and bg or fg)
                    end
                end
            else
                local p_red     = chain('fg', { 'DiagnosticError', 'ErrorMsg' },    { 'DiffDelete' })
                local p_green   = chain('fg', { 'DiagnosticOk', 'DiagnosticHint' }, { 'String', '@string' })
                local p_yellow  = chain('fg', { 'DiagnosticWarn', 'WarningMsg' },   { 'Number', '@number' })
                local p_blue    = chain('fg', { 'Function', '@function' },           { 'Directory' })
                local p_magenta = chain('fg', { 'Keyword', '@keyword' },             { 'Special' })
                local p_cyan    = chain('fg', { 'Type', '@type' },                   { 'Identifier' })
                local p_dim     = chain('fg', { 'Comment', '@comment' },             { 'NonText' })

                result = {
                    bg,
                    p_red,
                    p_green,
                    p_yellow,
                    p_blue,
                    p_magenta,
                    p_cyan,
                    p_dim,
                    p_dim,
                    p_red,
                    p_green,
                    p_yellow,
                    p_blue,
                    p_magenta,
                    p_cyan,
                    fg,
                }
            end
            return result
        end

        local white = '#ffffff'
        if vim.g.terminal_color_15 ~= nil then
            if type(vim.g.terminal_color_15) == 'string' then
                white = vim.g.terminal_color_15
            elseif type(vim.g.terminal_color_15) == 'number' then
                white = string.format('#%06x', vim.g.terminal_color_15)
            end
        else
            white = fg
        end

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

            white     = white,
            ansi      = resolve_ansi(),
        }

        io.write(vim.fn.json_encode(res))
        "##
    }

    /// Table with core and syntax colors
    pub fn core_and_syntax_colors(&self) {
        let core: [(&str, &String); 5] = [
            ("Background", &self.bg),
            ("Foreground", &self.fg),
            ("Selection ", &self.sel),
            ("Caret     ", &self.caret),
            ("Gutter    ", &self.gutter_fg),
        ];

        let syntax: [(&str, &String); 5] = [
            ("Keyword ", &self.keyword),
            ("Function", &self.func),
            ("String  ", &self.string),
            ("Constant", &self.constant),
            ("Variable", &self.variable),
        ];

        for i in 0..5 {
            self.render_row(core[i], syntax[i]);
        }
    }

    /// ANSI grid colors (terminal colors 0-15)
    pub fn ansi_grid(&self) {
        for row in 0..2 {
            print!("  ");
            for col in 0..8 {
                let idx: usize = row * 8 + col;
                let color: &String = &self.ansi[idx];
                let label: String = format!(" {:02} ", idx);

                print!("{}", label.on_color_code(color).black());
            }

            println!();
        }
    }

    /// Code snippet with palette colors
    pub fn preview_code(&self) {
        let indent: &str = "    ";

        println!(
            "\n  {}{} {}{} {} {} {}{}",
            indent,
            "const".color_code_fg(&self.keyword),
            "ID".color_code_fg(&self.constant),
            ":".color_code_fg(&self.operator),
            "u32".color_code_fg(&self.type_name),
            "=".color_code_fg(&self.operator),
            "2026".color_code_fg(&self.number),
            ";".color_code_fg(&self.operator)
        );
        println!();

        println!(
            "  {}{} {}{}{}{}{}{}{} {} {} {}",
            indent,
            "fn".color_code_fg(&self.keyword),
            "run".color_code_fg(&self.func),
            "(".color_code_fg(&self.gutter_fg),
            "s".color_code_fg(&self.variable),
            ":".color_code_fg(&self.operator),
            " &".color_code_fg(&self.operator),
            "str".color_code_fg(&self.type_name),
            ")".color_code_fg(&self.gutter_fg),
            "->".color_code_fg(&self.operator),
            "bool".color_code_fg(&self.type_name),
            "{".color_code_fg(&self.gutter_fg),
        );

        println!(
            "  {}{}  {} {}{}{}{} {} {} {} {} {}{}{}{}{}{}{}{}",
            indent,
            indent,
            "if".color_code_fg(&self.keyword),
            "s".color_code_fg(&self.variable),
            ".".color_code_fg(&self.operator),
            "len".color_code_fg(&self.func),
            "()".color_code_fg(&self.gutter_fg),
            "==".color_code_fg(&self.operator),
            "0".color_code_fg(&self.number),
            "{".color_code_fg(&self.gutter_fg),
            "return".color_code_fg(&self.keyword),
            format!("\"{}\"", "error").color_code_fg(&self.string),
            ".".color_code_fg(&self.operator),
            "contains".color_code_fg(&self.func),
            "(".color_code_fg(&self.gutter_fg),
            "s".color_code_fg(&self.variable),
            ")".color_code_fg(&self.gutter_fg),
            ";".color_code_fg(&self.operator),
            " }".color_code_fg(&self.gutter_fg)
        );

        println!(
            "  {}{}  {}{}{}{}",
            indent,
            indent,
            "Ok".color_code_fg(&self.type_name),
            "(".color_code_fg(&self.gutter_fg),
            "true".color_code_fg(&self.keyword),
            ")".color_code_fg(&self.gutter_fg),
        );

        print!("  {} {}", indent, "}".color_code_fg(&self.gutter_fg));
    }

    /// Helper function to render row in core vs syntax table
    fn render_row(&self, left: (&str, &String), right: (&str, &String)) {
        let format_col = |label: &str, hex: &str| {
            let (r, g, b) = utils::hex_to_rgb(hex);
            let rgb_str: String = format!("({},{},{})", r, g, b);
            let block = "  ".on_color_code(hex);

            format!(
                "{:<12} {}  {:<9} {:<15}",
                label.color_code_fg(&self.fg),
                block,
                hex.color_code_fg(&self.comment),
                rgb_str.bright_black()
            )
        };

        println!(
            "  {} │ {}",
            format_col(left.0, left.1),
            format_col(right.0, right.1)
        );
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
    use crate::core::tests::create_test_context;
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
        let args = Palette::build_fetch_args(theme, &ctx.state);

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
        ctx.state.manager = PluginManager::Lazy;
        let args = Palette::build_exists_args("gruvbox", &ctx.state);

        assert!(args.contains(&"--headless".to_string()));
        assert!(args.contains(&"NONE".to_string()));
        assert!(args.iter().any(|a| a.contains("cquit 1")));

        let has_rtp = args.iter().any(|a| a.contains("vim.opt.rtp:append"));
        assert!(has_rtp, "Lazy must include RTP setup in arguments");
    }

    #[test]
    fn should_test_build_args_without_rtp_for_default_manager() {
        let (_temp, mut ctx) = create_test_context();
        ctx.state.manager = PluginManager::Default;
        let args = Palette::build_exists_args("default_theme", &ctx.state);

        let has_rtp = args.iter().any(|a| a.contains("vim.opt.rtp:append"));
        assert!(!has_rtp, "Default manager should NOT include RTP setup");
    }

    #[test]
    fn should_read_from_cache_in_default_manager_even_if_external() {
        let (_temp, mut ctx) = create_test_context();
        ctx.state.manager = PluginManager::Default;
        let theme = "vesper";
        let cache_path = ctx.paths.palettes.join(format!("{}.json", theme));

        let dummy_palette = Palette {
            name: "vesper".to_string(),
            bg: "#ffffff".to_string(),
            ..serde_json::from_str(r##"{"bg":"#ffffff","fg":"","caret":"","line_hl":"","sel":"","gutter_fg":"","comment":"","variable":"","constant":"","number":"","string":"","keyword":"","operator":"","func":"","type_name":"","tag":"","attribute":"","white":"","ansi":[]}"##).unwrap()
        };
        Palette::save_to_cache(&cache_path, &dummy_palette).unwrap();

        let result = Palette::fetch(theme, false, false, &ctx.paths, &ctx.state, &ctx.log);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "vesper");
    }

    #[test]
    fn should_ignore_cache_when_force_is_true() {
        let (_temp, mut ctx) = create_test_context();
        ctx.state.manager = PluginManager::Lazy;

        let theme = "habamax";
        let cache_path = ctx.paths.palettes.join(format!("{}.json", theme));

        let old_palette = Palette {
            name: "old".to_string(),
            bg: "#000000".to_string(),
            ..serde_json::from_str(r##"{"bg":"#000000","fg":"","caret":"","line_hl":"","sel":"","gutter_fg":"","comment":"","variable":"","constant":"","number":"","string":"","keyword":"","operator":"","func":"","type_name":"","tag":"","attribute":"","white":"","ansi":[]}"##).unwrap()
        };
        Palette::save_to_cache(&cache_path, &old_palette).unwrap();

        let cached_res =
            Palette::fetch(theme, false, false, &ctx.paths, &ctx.state, &ctx.log).unwrap();
        assert_eq!(cached_res.name, "old");
        let forced_res = Palette::fetch(theme, true, false, &ctx.paths, &ctx.state, &ctx.log);

        match forced_res {
            Ok(p) => {
                assert_ne!(
                    p.name, "old",
                    "Should have fetched fresh data, not the old cache"
                );
            }
            Err(e) => {
                eprintln!("Bypassed cache: {}", e);
            }
        }
    }

    #[test]
    fn should_only_save_to_cache_when_save_flag_is_true() {
        let (_temp, ctx) = create_test_context();
        let theme = "tokyonight";
        let cache_path = ctx.paths.palettes.join(format!("{}.json", theme));

        assert!(!cache_path.exists());

        let _ = Palette::fetch(theme, false, false, &ctx.paths, &ctx.state, &ctx.log);
        assert!(
            !cache_path.exists(),
            "File should not be created if save = false"
        );
    }
}
