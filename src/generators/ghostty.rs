use crate::models::Theme;
use std::fs;

/// Generate ghostty config based on selected theme
pub fn apply(theme: &Theme) {
    let home = dirs::home_dir().expect("Home dir not found");
    let ghostty_dir = home.join(".config/ghostty");
    let cache_file = home.join(".cache/iris/ghostty.conf");

    let mut cfg = String::new();
    let fix = |v: &String| {
        if v.starts_with('#') {
            v.clone()
        } else {
            format!("#{}", v)
        }
    };

    for (k, v) in &theme.colors {
        let key = if k == "cursor" { "cursor-color" } else { k };
        cfg.push_str(&format!("{} = {}\n", key, fix(v)));
    }

    let mut palette: Vec<_> = theme.palette.iter().collect();
    palette.sort_by_key(|(k, _)| k.parse::<u32>().unwrap_or(0));
    for (idx, val) in palette {
        cfg.push_str(&format!("palette = {}={}\n", idx, fix(val)));
    }

    fs::create_dir_all(cache_file.parent().unwrap()).ok();
    fs::write(&cache_file, cfg).ok();

    let link = ghostty_dir.join("current_theme.conf");
    let _ = fs::remove_file(&link);
    let _ = std::os::unix::fs::symlink(&cache_file, &link);
}
