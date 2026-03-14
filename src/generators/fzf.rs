/// Build colors for fzf based on current palette
pub fn build_fzf_args(p: &crate::models::Palette) -> String {
    let c = |hex: &str| hex.trim_start_matches('#').to_string();

    let fg = c(&p.fg);
    let accent = c(&p.ansi[3]);
    let match_c = c(&p.ansi[5]);
    let dimmed = c(&p.ansi[8]);

    format!(
        "bg:-1,fg:#{fg},\
        bg+:-1,fg+:#{accent}:bold,\
        hl:#{match_c},hl+:#{match_c}:underline,\
        pointer:#{accent},info:#{dimmed},border:#{dimmed},\
        prompt:#{accent},marker:#{accent},spinner:#{match_c}",
        fg = fg,
        accent = accent,
        match_c = match_c,
        dimmed = dimmed
    )
}
