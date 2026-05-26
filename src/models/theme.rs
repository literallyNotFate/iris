use crate::utils::{self, CustomColor};
use anyhow::{Context, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

/// Main theme entity in Iris: color palette with unique name
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Theme {
    pub name: String,
    pub colors: Palette,
}

/// Clean color palette that is being returned from Neovim
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Palette {
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

impl Theme {
    pub fn new(name: impl Into<String>, palette: Palette) -> Self {
        Self {
            name: name.into(),
            colors: palette,
        }
    }

    /// Fast method to create a theme from palette
    pub fn from_palette(name: &str, palette: Palette) -> Self {
        Self {
            name: name.to_string(),
            colors: palette,
        }
    }

    /// Load theme from JSON cache file
    pub fn load_from_cache(path: &Path) -> Result<Self> {
        let content: String = fs::read_to_string(path)
            .with_context(|| format!("Failed to read theme cache at {}", path.display()))?;

        let theme: Self = serde_json::from_str(&content)
            .with_context(|| format!("Failed to deserialize theme JSON from {}", path.display()))?;

        Ok(theme)
    }

    /// Save theme to JSON cache file, automatically creating all necessary folders
    pub fn save_to_cache(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create cache directory: {}", parent.display())
            })?;
        }

        let json: String =
            serde_json::to_string_pretty(self).context("Failed to serialize theme to JSON")?;
        fs::write(path, json)
            .with_context(|| format!("Failed to write theme to {}", path.display()))?;

        Ok(())
    }

    /// Helper to get capitalized theme name (e.g, "melange" -> "Melange")
    pub fn display_name(&self) -> String {
        utils::capitalize(&self.name)
    }

    /// Function to create theme mock
    #[cfg(test)]
    pub fn mock() -> Self {
        Self {
            name: "test-theme".into(),
            colors: Palette {
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
            },
        }
    }
}

impl Palette {
    /// Lua script to fetch palette from nvim
    pub fn fetch_lua_script() -> &'static str {
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
                let color: &String = self.ansi.get(idx).unwrap_or(&self.fg);
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
}

/// Unit-tests for palette operation
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::tests::create_test_context;

    #[test]
    fn should_parse_palette_json_without_name() {
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
    fn should_properly_link_name_and_palette() {
        let palette: Palette = Theme::mock().colors;
        let theme: Theme = Theme::new("tokyonight", palette.clone());

        assert_eq!(theme.name, "tokyonight");
        assert_eq!(theme.colors.bg, palette.bg);
    }

    #[test]
    fn should_serialize_theme_with_nested_palette() {
        let theme: Theme = Theme::mock();
        let json: String = serde_json::to_string(&theme).unwrap();

        assert!(json.contains(r#""name":"test-theme""#));
        assert!(json.contains(r#""bg":"#));
    }

    #[test]
    fn should_save_and_load_theme_from_cache() {
        let (_temp, ctx) = create_test_context();
        let cache_path = ctx.paths.themes.join("catppuccin.json");
        let original_theme = Theme::mock();
        let save_res = original_theme.save_to_cache(&cache_path);

        assert!(
            save_res.is_ok(),
            "Failed to save theme: {:?}",
            save_res.err()
        );
        assert!(cache_path.exists());

        let load_res = Theme::load_from_cache(&cache_path);
        assert!(
            load_res.is_ok(),
            "Failed to load theme: {:?}",
            load_res.err()
        );

        let loaded_theme = load_res.unwrap();
        assert_eq!(loaded_theme, original_theme);
    }

    #[test]
    fn should_fail_on_non_existent_cache() {
        let (_temp, ctx) = create_test_context();
        let ghost_path = ctx.paths.themes.join("ghost_theme.json");
        let result = Theme::load_from_cache(&ghost_path);

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Failed to read theme cache")
        );
    }

    #[test]
    fn should_fail_on_broken_json() {
        let (_temp, ctx) = create_test_context();
        let cache_path = ctx.paths.themes.join("broken_theme.json");
        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&cache_path, "{ broken json }").unwrap();

        let result = Theme::load_from_cache(&cache_path);

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Failed to deserialize theme JSON")
        );
    }
}
