# Iris

**Iris** is a fast, minimalist theme orchestration tool for Linux and macOS, written in Rust. It acts as the central bridge between your Neovim colorscheme and the rest of your system (terminals, CLI tools, and desktop components).

**Warning**: I made this project for my personal workflow so it is kinda experimental. If you are ready to tweak your environment or fork the app itself — here you go, but be careful :)

## Features

- **Rust-Powered**: Near-instant execution with zero overhead.

- **Neovim Sync**: Extracts and propagates colors from your active Neovim session to the whole system.

- **Smart Registry**: Categorized management for Terminal, Tool, Prompt, Multiplexer and System generators.

- **Advanced Filtering**: Search through your generators by type (-t) or status (-s).

- **Polished CLI**: Interactive selection menus with Nerd Font icons and real-time status indicators using [dialoguer](https://github.com/console-rs/dialoguer)

- **Dynamic Shell Sync**: A lightweight `zsh` hook that updates your active terminal sessions without restarts.

## Prerequisites

#### Neovim Integration

For Iris to follow your Neovim theme, you need to tell Neovim to "broadcast" its current colorscheme name. Add this snippet to your `init.lua`:

```lua
vim.api.nvim_create_autocmd("ColorScheme", {
    desc = "Notify Iris about theme change",
    callback = function()
        local theme = vim.g.colors_name
        if theme then
            local cache_dir = vim.fn.expand("~/.cache/iris/")
            if vim.fn.isdirectory(cache_dir) == 0 then
                vim.fn.mkdir(cache_dir, "p")
            end
            local f = io.open(cache_dir .. "current_theme", "w")
            if f then
                f:write(theme)
                f:close()
            end
        end
    end,
})
```

#### Nerd Fonts

The CLI UI heavily relies on specific glyphs. Ensure your terminal is using a Nerd Font.

#### Basic zsh configuration

To make your apps reactive to Iris, update your configs as follows:

Add these lines to your `.zshrc`:
```bash
# Sync fzf colors with Iris
[ -f ~/.cache/iris/fzf.sh ] && source ~/.cache/iris/fzf.sh

# Bind to it fzf
export FZF_DEFAULT_OPTS="$FZF_DEFAULT_OPTS"

# Tell bat to use Iris-generated config
export BAT_CONFIG_PATH="$HOME/.cache/iris/bat.conf"
```

## Initial Setup
1. **Installation**

Build from source using *Cargo*:

```bash
git clone https://github.com/literallyNotFate/iris.git
cd iris
cargo install --path .
```

2. **Initialization**

This is the most important step. It prepares the configuration ecosystem:

```bash
iris init
```

What init does:

- Creates `~/.config/iris/` directory.

- Creates `~/.cache/iris/` directory.

- Initializes state file to track your preferences.

- Injects a Sync Hook into your `.zshrc`. This hook allows Iris to update the environment (like `fzf` colors) in real-time across all open terminal tabs whenever you switch themes.

3. **Generator Discovery**

Tell Iris to scan your system for supported applications:
```bash
iris gen auto
```

## Usage
1. **Interactive Management**

Toggle which applications Iris should manage:
```bash
iris gen select
```

2. **Listing & Filtering**

Check your registry with granular control:
```bash
# List only active terminals
iris gen list --type terminal --status active

# Find "broken" generators (enabled but app not found in PATH)
iris gen list -s broken

# Show everything in a compact, quiet mode (useful for scripts)
iris gen list --quiet
```

3. **Theming**
```bash
# Switch to a specific theme manually
iris switch melange

# Sync all enabled apps with your current Neovim state
iris sync

# Check the current global state (active theme and enabled counts)
iris status
```
