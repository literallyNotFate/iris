use crate::models::Theme;
use std::fs;

pub fn apply(theme: &Theme) {
    let home = dirs::home_dir().expect("Home dir not found");
    let cache_dir = home.join(".cache/iris");
    let cache_file = cache_dir.join("fzf.sh");

    let bg = theme
        .colors
        .get("background")
        .cloned()
        .unwrap_or("#292522".into());
    let fg = theme
        .colors
        .get("foreground")
        .cloned()
        .unwrap_or("#ece1d7".into());

    let black = theme.palette.get("0").cloned().unwrap_or("#34302c".into());
    let red = theme.palette.get("1").cloned().unwrap_or("#c6735a".into());
    let green = theme.palette.get("2").cloned().unwrap_or("#78997a".into());
    let yellow = theme.palette.get("3").cloned().unwrap_or("#bb9751".into());
    let magenta = theme.palette.get("5").cloned().unwrap_or("#b38d6b".into());
    let br_black = theme.palette.get("8").cloned().unwrap_or("#8b5939".into());

    let fzf_colors = format!(
        "bg:#{bg},\
        fg:#{fg},\
        bg+:#{bg_plus},\
        fg+:#{fg},\
        hl:#{red},\
        hl+:#{yellow},\
        header:#{green},\
        info:#{yellow},\
        pointer:#{fg},\
        marker:#{red},\
        prompt:#{green},\
        query:#{fg},\
        spinner:#{magenta},\
        disabled:#{br_black},\
        border:#{border},\
        scrollbar:#{br_black},\
        label:#{fg},\
        preview-label:#{fg}",
        bg = clean_hex(&bg),
        bg_plus = clean_hex(&black),
        fg = clean_hex(&fg),
        red = clean_hex(&red),
        green = clean_hex(&green),
        yellow = clean_hex(&yellow),
        magenta = clean_hex(&magenta),
        border = clean_hex(&br_black),
    );

    let content = format!("export FZF_DEFAULT_OPTS=\"$FZF_DEFAULT_OPTS --color='{fzf_colors}'\"");

    fs::create_dir_all(&cache_dir).ok();
    fs::write(&cache_file, content).expect("Failed to write FZF cache");
}

fn clean_hex(s: &str) -> String {
    s.trim_start_matches('#').to_string()
}
