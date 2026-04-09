use crate::ui::Logger;
use anyhow::{Context, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::process::Command;

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
        let home = dirs::home_dir().context("Home dir not found")?;
        let path = home.join(".cache/iris/current_theme");

        if path.exists() {
            let name: String = std::fs::read_to_string(&path)
                .with_context(|| format!("Failed to read theme cache at {:?}", path))?;

            let trimmed: &str = name.trim();
            if trimmed.is_empty() {
                log.warn("Theme cache file is empty. Please set a theme first.", 1);
                anyhow::bail!("Theme cache is empty");
            }

            let capitalized = trimmed[..1].to_uppercase() + &trimmed[1..];
            return Ok(capitalized);
        }

        log.error("No active theme found in cache.", 1);
        anyhow::bail!(
            "No active theme detected.\n\
             {} Make sure to switch theme in Neovim (e.g, :colorscheme <theme>) \n\
             or pass the name manually: `iris switch <name>`",
            "Tip:".yellow()
        );
    }

    /// Fetch palette from nvim using lua script
    pub fn fetch(theme: &str, log: &Logger) -> Result<Self> {
        log.info("Loading Neovim runtime and lazy.nvim plugins...");

        let lua_script = r##"
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
                        if c ~= '#cccccc' then return c end
                    end
                    return '#cccccc'
                end

                local ansi = {}
                for i = 0, 15 do
                    table.insert(ansi, vim.g['terminal_color_' .. i] or '#000000')
                end

                local res = {
                    bg       = g('Normal', 'bg'),
                    fg       = g('Normal', 'fg'),
                    caret    = first('bg', {'Cursor', 'TermCursor'}),
                    line_hl  = first('bg', {'CursorLine', 'CursorLineBg'}),
                    sel      = first('bg', {'Visual', 'Selection'}),
                    gutter_fg = first('fg', {'LineNr', 'SignColumn'}),
                    comment  = first('fg', {'Comment', '@comment'}),
                    variable = first('fg', {'@variable', 'Identifier'}),
                    constant = first('fg', {'Constant', '@constant'}),
                    number   = first('fg', {'Number', '@number', 'Constant'}),
                    string   = first('fg', {'String', '@string'}),
                    keyword  = first('fg', {'Keyword', '@keyword', 'Statement'}),
                    operator = first('fg', {'Operator', '@operator'}),
                    func     = first('fg', {'Function', '@function'}),
                    type_name = first('fg', {'Type', '@type'}),
                    tag      = first('fg', {'Tag', '@tag'}),
                    attribute = first('fg', {'@attribute', '@property'}),
                    added     = first('fg', {'DiffAdd', 'GitSignsAdd', '@diff.plus'}),
                    deleted   = first('fg', {'DiffDelete', 'GitSignsDelete', '@diff.minus'}),
                    changed   = first('fg', {'DiffChange', 'GitSignsChange', '@diff.delta'}),
                    white    = vim.g.terminal_color_15 or '#ffffff',
                    ansi     = ansi,
                }
                io.write(vim.fn.json_encode(res))
            "##;

        log.info("Executing Lua bridge in headless mode...");

        let output = Command::new("nvim")
            .args([
                "--headless",
                "-u",
                "NONE",
                "-c",
                "lua vim.opt.rtp:append(vim.fn.stdpath('data') .. '/lazy/*')",
                "-c",
                &format!("colorscheme {}", theme.to_lowercase()),
                "-c",
                &format!("lua {}", lua_script),
                "-c",
                "qa!",
            ])
            .output()?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            log.error(&format!("Neovim error: {}", error_msg.trim()), 2);
            anyhow::bail!("Neovim error: {}", error_msg.red());
        }

        log.info("Parsing palette data...");
        let stdout = String::from_utf8_lossy(&output.stdout);

        let json_start = stdout.find('{').context("Nvim did not return JSON")?;
        let json_end = stdout.rfind('}').context("JSON is malformed")? + 1;

        let mut palette: Palette = serde_json::from_str(&stdout[json_start..json_end])
            .context("Failed to parse palette JSON")?;
        palette.name = theme.to_string();

        Ok(palette)
    }

    /// Checks whether this theme exists in nvim colorscheme
    pub fn exists(theme: &str, log: &Logger) -> bool {
        if which::which("nvim").is_err() {
            log.warn("Neovim not found. Skipping theme verification.", 0);
            return false;
        }

        let mut check_task = log.step(
            &format!("Checking theme availability: {}", theme.yellow().bold()),
            1,
        );

        let init_plugins = "lua vim.opt.rtp:append(vim.fn.stdpath('data') .. '/lazy/*')";
        let check_cmd = format!(
            "try | colorscheme {} | qa! | catch | cquit 1 | endtry",
            theme.to_lowercase()
        );

        let output = std::process::Command::new("nvim")
            .args([
                "--headless",
                "-u",
                "NONE",
                "-c",
                init_plugins,
                "-c",
                &check_cmd,
            ])
            .output();

        let success = match output {
            Ok(o) => o.status.success(),
            Err(_) => false,
        };

        if success {
            check_task.done(true);
        } else {
            log.error(&format!("Theme {} not found in Neovim", theme.red()), 2);
        }

        success
    }
}
