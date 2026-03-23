use crate::{context::AppContext, models::Palette, status::Status};
use anyhow::{Context, Result};
use std::{fs, path::PathBuf, process::Command};

/// Apply generated .tmTheme to current bat theme
pub fn apply(palette: &Palette, ctx: &AppContext) -> Result<()> {
    let iris_bat_dir = ctx.cache_path.join("bat_themes");
    fs::create_dir_all(&iris_bat_dir)?;

    let config_task = Status::step("Locating bat configuration...", 2);
    let bat_config_dir = Command::new("bat")
        .arg("--config-dir")
        .output()
        .map(|o| PathBuf::from(String::from_utf8_lossy(&o.stdout).trim()))
        .context("Failed to get bat config dir")?;

    let bat_themes_dir = bat_config_dir.join("themes");
    let link_path = bat_themes_dir.join("iris_themes");

    if !bat_themes_dir.exists() {
        fs::create_dir_all(&bat_themes_dir)?;
    }
    config_task.done(Some("Bat configuration directory located!"));

    if !link_path.exists() {
        #[cfg(unix)]
        {
            let link_task = Status::step("Linking iris themes to bat...", 2);
            std::os::unix::fs::symlink(&iris_bat_dir, &link_path)?;
            link_task.done(Some("Symlink created."));
        }
    }

    let theme_name = &ctx.state.current_theme;
    let theme_dir = ctx.cache_path.join("bat_themes");
    std::fs::create_dir_all(&theme_dir).context("Failed to create bat theme directory")?;

    let rules = crate::generators::bat::build_bat_tm_theme(palette);

    let content = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple Computer//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>name</key><string>{name}</string>
    <key>settings</key>
    <array>
{rules}
    </array>
</dict>
</plist>"#,
        name = theme_name,
        rules = rules
    );

    let theme_file = theme_dir.join(format!("{}.tmTheme", theme_name));
    std::fs::write(&theme_file, content).context("Failed to write theme file")?;

    let config_file = ctx.cache_path.join("bat.conf");
    let bat_config = format!(
        "--theme=\"{name}\"\n--style=\"numbers,changes\"\n--color=\"always\"\n",
        name = theme_name
    );
    std::fs::write(config_file, bat_config).context("Failed to write bat.conf")?;

    let cache_task = Status::step("Rebuilding bat cache...", 2);
    let output = std::process::Command::new("bat")
        .arg("cache")
        .arg("--build")
        .output()?;

    if output.status.success() {
        cache_task.done(Some("Bat cache updated."));
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        cache_task.fail(&err);
        anyhow::bail!("Bat cache build failed");
    }

    Ok(())
}
