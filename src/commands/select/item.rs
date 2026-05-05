/// Theme to show in the list
pub struct ThemeItem {
    pub name: String,
    pub is_cached: bool,
    pub is_builtin: bool,
    pub is_active: bool,
    pub is_fallback: bool,
}

impl ThemeItem {
    pub fn render_label(&self) -> String {
        use colored::*;

        let name_col = if self.is_active {
            self.name.green().bold()
        } else {
            self.name.normal()
        };

        format!(
            "{:<25} {:<12} {:<12} {}",
            name_col,
            if self.is_cached {
                "[cached]".dimmed()
            } else {
                "[remote]".yellow().dimmed()
            },
            if self.is_builtin {
                "[builtin]".bright_red()
            } else {
                "[lazy]".bright_cyan()
            },
            if self.is_active {
                "󰄬  active".green()
            } else if self.is_fallback {
                "󰁯  fallback".magenta()
            } else {
                "".normal()
            }
        )
    }
}
