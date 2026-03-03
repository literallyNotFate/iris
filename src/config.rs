use crate::models::UIState;
use colored::*;
use std::path::PathBuf;
use std::{fs, io::Write};

/// Default theme
const MELANGE_CONTENT: &str = include_str!("../defaults/melange.toml");

/// Get base .config path
pub fn get_base_path() -> PathBuf {
    std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| dirs::home_dir().unwrap().join(".config"))
        .join("iris")
}

/// Get state.json path
pub fn get_state() -> UIState {
    let path = get_base_path().join("state.json");
    fs::read_to_string(path)
        .ok()
        .and_then(|c| serde_json::from_str(&c).ok())
        .unwrap_or_default()
}

/// Initialize project on "iris init"
pub fn init_project() {
    let base = get_base_path();
    let themes_dir = base.join("themes");
    let cache_dir = dirs::home_dir().unwrap().join(".cache/iris");

    println!(
        "\n{}",
        "Initializing Iris environment...".bright_blue().bold()
    );

    fs::create_dir_all(&themes_dir).ok();
    fs::create_dir_all(&cache_dir).ok();
    println!("  {} Directories created", "✔".green());

    deploy_theme(&themes_dir, "melange.toml", MELANGE_CONTENT);

    setup_initial_state(&base);
    setup_zsh_hook();

    println!(
        "\n{}",
        "Setup complete! Default theme (Melange) is ready.".green()
    );
    println!("Type {} to get started.", "iris switch melange".cyan());
}

/// Deploying theme from defaults
fn deploy_theme(dir: &PathBuf, filename: &str, content: &str) {
    let path = dir.join(filename);
    if !path.exists() {
        if fs::write(&path, content).is_ok() {
            println!(
                "  {} Deployed default theme: {}",
                "✔".green(),
                filename.yellow()
            );
        }
    }
}

/// Setup state.json
fn setup_initial_state(base_path: &PathBuf) {
    let state_path = base_path.join("state.json");
    if state_path.exists() {
        return;
    }

    let mut enabled = Vec::new();
    let home = dirs::home_dir().unwrap();

    if home.join(".config/ghostty").exists() {
        enabled.push("ghostty".to_string());
    }
    if home.join(".zshrc").exists() || home.join(".config/zsh").exists() {
        enabled.push("fzf".to_string());
    }

    let initial_state = UIState {
        current_theme: "melange".to_string(),
        enabled_generators: enabled,
    };

    fs::write(
        state_path,
        serde_json::to_string_pretty(&initial_state).unwrap(),
    )
    .ok();
}

/// ZSH hook to track changes
pub fn setup_zsh_hook() {
    let zshrc = dirs::home_dir().unwrap().join(".zshrc");
    if !zshrc.exists() {
        return;
    }

    let hook_identifier: &str = "# --- Iris FZF Sync ---";
    if let Ok(content) = fs::read_to_string(&zshrc) {
        if content.contains(hook_identifier) {
            println!("  {} Zsh hook already exists, skipping", "ℹ".blue());
            return;
        }
    }

    let hook = r#"
    # --- Iris FZF Sync ---
    autoload -Uz add-zsh-hook
    _iris_fzf_sync() {
        local cache_file="$HOME/.cache/iris/fzf.sh"
        if [[ -f "$cache_file" ]]; then
            local mt=$(stat -f %m "$cache_file" 2>/dev/null || stat -c %Y "$cache_file" 2>/dev/null)
            if [[ "$mt" != "$LAST_IRIS_SYNC" ]]; then
                source "$cache_file"
                export LAST_IRIS_SYNC="$mt"
            fi
        fi
    }
    add-zsh-hook precmd _iris_fzf_sync
    # ---------------------
    "#;

    let mut file = fs::OpenOptions::new().append(true).open(&zshrc).unwrap();
    if writeln!(file, "{}", hook).is_ok() {
        println!("  {} Successfully injected Zsh hook", "✔".green());
    }
}

/// Save current UIState to JSON file
pub fn save_state(theme_name: &str) {
    let base = get_base_path();
    let path = base.join("state.json");

    let mut state: UIState = fs::read_to_string(&path)
        .ok()
        .and_then(|c| serde_json::from_str(&c).ok())
        .unwrap_or_else(|| UIState {
            current_theme: theme_name.to_string(),
            enabled_generators: vec!["ghostty".into(), "fzf".into()],
        });

    state.current_theme = theme_name.to_string();
    fs::write(path, serde_json::to_string_pretty(&state).unwrap()).ok();
}
