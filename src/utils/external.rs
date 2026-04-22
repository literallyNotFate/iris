use std::process::Command;

/// Clears the cache of bat (if installed)
pub fn clear_bat_cache() {
    let bin: Option<&str> = if which::which("bat").is_ok() {
        Some("bat")
    } else if which::which("batcat").is_ok() {
        Some("batcat")
    } else {
        None
    };

    if let Some(bat_bin) = bin {
        let _ = Command::new(bat_bin).args(["cache", "--clear"]).status();
    }
}
