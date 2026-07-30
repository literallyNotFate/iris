use crate::{
    infra::RESOURCES_DIR,
    utils::{self, CustomColor},
};
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
    pub added: String,
    pub deleted: String,
    pub changed: String,
    pub ansi: Vec<String>,
}

impl Theme {
    pub fn new(name: impl Into<String>, palette: Palette) -> Self {
        Self {
            name: name.into(),
            colors: palette,
        }
    }

    /// Load theme from JSON cache file
    pub fn load_from_cache(path: &Path) -> Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }

        let content: String = fs::read_to_string(path)
            .with_context(|| format!("Failed to read theme cache at {}", path.display()))?;

        let theme: Self = serde_json::from_str(&content)
            .with_context(|| format!("Failed to deserialize theme JSON from {}", path.display()))?;

        Ok(Some(theme))
    }

    /// Save theme to JSON cache file, automatically creating all necessary folders
    pub fn save_to_cache(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create cache directory: {}", parent.display())
            })?;
        }

        let json: String = self
            .to_json()
            .context("Failed to serialize theme to JSON")?;
        fs::write(path, json)
            .with_context(|| format!("Failed to write theme to {}", path.display()))?;

        Ok(())
    }

    /// Helper to get capitalized theme name (e.g, "melange" -> "Melange")
    pub fn display_name(&self) -> String {
        utils::capitalize(&self.name)
    }

    /// Serialize the theme into a pretty-printed JSON string
    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(self)
    }

    /// Compact variant, if you don't need pretty-printing
    pub fn to_json_compact(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
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
                added: "#9ece6a".into(),
                deleted: "#f7768e".into(),
                changed: "#e0af68".into(),
                ansi: (0..16).map(|_| "#ffffff".to_string()).collect(),
            },
        }
    }
}

impl Palette {
    /// Lua script to fetch palette from nvim
    pub fn fetch_lua_script() -> &'static str {
        RESOURCES_DIR
            .get_file("lua/fetch_palette.lua")
            .expect("fetch_palette.lua must be included")
            .contents_utf8()
            .expect("File must be valid utf8")
    }

    /// Helper to clear stdout from Neovim garbage and extract JSON palette
    pub fn parse_from_nvim(stdout: &str) -> Result<Palette> {
        let json_start: usize = stdout
            .find('{')
            .context("Failed to locate opening brace '{' of palette JSON within `nvim` output")?;

        let mut deserializer = serde_json::Deserializer::from_str(&stdout[json_start..]);
        let palette = Palette::deserialize(&mut deserializer)
            .context("Failed to parse palette JSON within `nvim` output")?;

        Ok(palette)
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
    use crate::core::IrisContext;

    #[test]
    fn should_parse_theme_from_json() {
        let json = r##"{
            "name": "test",
            "colors": {
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
                "added": "#ffffff",
                "changed": "#ffffff",
                "deleted": "#ffffff",
                "ansi": ["#1e1e2e", "#f38ba8", "#a6e3a1", "#f9e2af", "#89b4fa", "#cba6f7", "#89dceb", "#bac2de", "#585b70", "#f38ba8", "#a6e3a1", "#f9e2af", "#89b4fa", "#cba6f7", "#89dceb", "#a6adc8"]
            }
        }"##;

        let theme: Theme = serde_json::from_str(json).unwrap();
        assert_eq!(theme.name, "test");
        assert_eq!(theme.colors.bg, "#1e1e2e");
        assert_eq!(theme.colors.attribute, "#f9e2af");
        assert_eq!(theme.colors.ansi.len(), 16);
    }

    #[test]
    fn should_save_and_load_theme_from_cache() {
        let (_temp, ctx) = IrisContext::mock();
        let cache_path = ctx.paths.themes.join("catppuccin.json");
        let original_theme = Theme::mock();

        let save_res = original_theme.save_to_cache(&cache_path);
        assert!(save_res.is_ok(), "Failed to save: {:?}", save_res.err());
        assert!(cache_path.exists());

        let load_res = Theme::load_from_cache(&cache_path);
        assert!(load_res.is_ok(), "Failed to load: {:?}", load_res.err());

        let loaded_theme = load_res.unwrap().expect("Should return Some(theme)");
        assert_eq!(loaded_theme, original_theme);
    }

    #[test]
    fn should_return_none_on_non_existent_cache() {
        let (_temp, ctx) = IrisContext::mock();
        let ghost_path = ctx.paths.themes.join("ghost_theme.json");

        let result = Theme::load_from_cache(&ghost_path);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn should_fail_on_broken_json() {
        let (_temp, ctx) = IrisContext::mock();
        let cache_path = ctx.paths.themes.join("broken_theme.json");
        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&cache_path, "{ broken json }").unwrap();

        let result = Theme::load_from_cache(&cache_path);
        assert!(result.is_err());
    }
}
