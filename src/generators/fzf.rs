use crate::{context::AppContext, models::Theme};
use anyhow::{Context as _, Result};

pub fn apply(theme: &Theme, ctx: &AppContext) -> Result<()> {
    let cache_file = ctx.fzf_cache_path();

    let get = |map: &std::collections::BTreeMap<String, String>, key: &str, default: &str| {
        let val = map.get(key).map(|s| s.as_str()).unwrap_or(default);
        val.trim_start_matches('#').to_string()
    };

    let fg = get(&theme.colors, "foreground", "ece1d7");
    let accent = get(&theme.palette, "3", "bb9751");
    let match_c = get(&theme.palette, "5", "c6735a");
    let dimmed = get(&theme.palette, "8", "34302c");

    let fzf_colors = format!(
        "bg:-1,fg:#{fg},\
            bg+:-1,fg+:#{accent}:bold,\
            hl:#{match_c},hl+:#{match_c}:underline,\
            pointer:#{accent},info:#{dimmed},border:#{dimmed},\
            prompt:#{accent},marker:#{accent},spinner:#{match_c}",
        fg = fg,
        accent = accent,
        match_c = match_c,
        dimmed = dimmed
    );

    let content: String =
        format!("export FZF_DEFAULT_OPTS=\"$FZF_DEFAULT_OPTS --color='{fzf_colors}'\"");

    std::fs::write(&cache_file, content)
        .with_context(|| format!("Failed to write FZF config to {:?}", cache_file))?;

    Ok(())
}
