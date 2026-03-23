use crate::status::Status;
use anyhow::{Context, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::process::Command;

/// Theme palette that is being retreived from nvim
#[derive(Debug, Deserialize, Serialize)]
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

impl Palette {
    /// Get currently installed theme name
    pub fn current() -> Result<String> {
        let home = dirs::home_dir().context("Home dir not found")?;
        let path = home.join(".cache/iris/current_theme");

        if path.exists() {
            let name: String = std::fs::read_to_string(&path)
                .with_context(|| format!("Failed to read theme cache at {:?}", path))?;

            let trimmed: &str = name.trim();
            if trimmed.is_empty() {
                Status::error("Theme cache file is empty.", 1);
                anyhow::bail!("Theme cache is empty");
            }

            let capitalized = trimmed[..1].to_uppercase() + &trimmed[1..];
            return Ok(capitalized);
        }

        Status::error("No active theme found in cache.", 1);
        anyhow::bail!(
            "No active theme detected.\n\
             {} Make sure to switch theme in Neovim (e.g, :colorscheme <theme>) \n\
             or pass the name manually: `iris switch <name>`",
            "Tip:".yellow()
        );
    }

    /// Fetch palette from nvim using lua script
    pub fn fetch(theme: &str) -> Result<Self> {
        let sync_task = Status::step(
            &format!(
                "Fetching colors from Neovim using theme: {}",
                theme.bold().cyan()
            ),
            1,
        );

        let lua_script = r##"
                local function g(name, attr)
                    local hl = vim.api.nvim_get_hl(0, { name = name, link = true })
                    local color = hl[attr or 'fg'] or hl.bg
                    return color and string.format('#%06x', color) or '#cccccc'
                end
                local ansi = {}
                for i=0,15 do
                    table.insert(ansi, vim.g['terminal_color_'..i] or '#000000')
                end
                local res = {
                    bg = g('Normal', 'bg'),
                    fg = g('Normal'),
                    caret = g('Cursor'),
                    line_hl = g('CursorLine', 'bg'),
                    sel = g('Visual', 'bg'),
                    gutter_fg = g('LineNr'),
                    comment = g('Comment'),
                    variable = g('@variable'),
                    constant = g('Constant'),
                    number = g('Number'),
                    string = g('String'),
                    keyword = g('Keyword'),
                    operator = g('Operator'),
                    func = g('Function'),
                    type_name = g('Type'),
                    tag = g('Tag'),
                    attribute = g('@attribute'),
                    link = g("Underlined"),
                    heading = g("Title"),
                    added = g("DiffAdd"),
                    deleted = g("DiffDelete"),
                    changed = g("DiffChange"),
                    white = vim.g.terminal_color_15 or '#ffffff',
                    ansi = ansi
                }
                io.write(vim.fn.json_encode(res))
            "##;

        let nvim_task = Status::step("Executing Lua bridge in headless mode...", 2);

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
            nvim_task.fail("Neovim failed to provide theme data");
            sync_task.fail("Synchronization failed");
            anyhow::bail!("Neovim error: {}", error_msg.red());
        }

        nvim_task.done(Some("Lua bridge executed successfully."));
        let stdout = String::from_utf8_lossy(&output.stdout);

        let json_start = stdout.find('{').context("Nvim did not return JSON")?;
        let json_end = stdout.rfind('}').context("JSON is malformed")? + 1;

        let palette: Palette = serde_json::from_str(&stdout[json_start..json_end])
            .context("Failed to parse palette JSON")?;

        sync_task.done(Some("Palette successfully synchronized with Neovim."));
        Ok(palette)
    }

    /// Checks whether this theme exists in nvim colorscheme
    pub fn exists(theme: &str) -> bool {
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

        match output {
            Ok(o) => o.status.success(),
            Err(_) => false,
        }
    }
}
