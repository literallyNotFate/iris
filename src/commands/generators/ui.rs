use crate::modules::Generator;
use colored::Colorize;

/// Helper function to render header for generators
pub(super) fn render_header(title: &str, icon: &str) {
    println!("\n {}  {}\n", icon.green().bold(), title.bold());
}

/// Helper function to render list header for generators list
pub(super) fn render_list_header(is_filtered: bool) {
    let suffix = if is_filtered {
        "(filtered)".dimmed().italic()
    } else {
        "".into()
    };
    println!(
        "\n {}  {} {}",
        "󰒓".yellow().bold(),
        "Registry of Generators".bold(),
        suffix
    );
    println!(
        "\n    {:<20}  {:<14}  {}",
        "NAME".dimmed(),
        "TYPE".dimmed(),
        "STATUS".dimmed()
    );
    println!();
}

/// Helper function to render full row for generators list
pub(super) fn render_full_row(g: &dyn Generator, enabled: bool, installed: bool) {
    let g_type = g.generator_type();

    let (icon, status_label, name_style) = match (enabled, installed) {
        (true, true) => (
            "󰄬 ".green(),
            "active".green().italic(),
            g.name().cyan().bold(),
        ),
        (true, false) => (
            "󰀦 ".yellow(),
            "broken".yellow(),
            g.name().yellow().strikethrough(),
        ),
        (false, true) => ("󰈈 ".dimmed(), "ready".dimmed(), g.name().normal()),
        (false, false) => ("󰂭 ".red(), "missing".red(), g.name().normal()),
    };

    println!(
        "  {} {:<17} │ {} {:<11} │ {}",
        icon,
        name_style,
        g_type.icon().color(g_type.color()),
        g_type.label().color(g_type.color()),
        status_label
    );
}

/// Helper function to render quiet row
pub(super) fn render_quiet_row(g: &dyn Generator, enabled: bool) {
    let prefix = if enabled { "+".bold() } else { "-".dimmed() };
    println!(
        "{} {:<14} ({})",
        prefix,
        g.name(),
        g.generator_type().label()
    );
}

/// Helper function to render generator list footer
pub(super) fn render_list_footer(total: usize, enabled: usize, no_filter: bool) {
    println!(
        "{}",
        " ──────────────────────────────────────────────────────────".dimmed()
    );
    println!(
        " {} {} {} {} {} {}",
        "󰛵".blue(),
        "Showing:".dimmed(),
        total.to_string().bold().cyan(),
        "generators,".dimmed(),
        enabled.to_string().bold().green(),
        "enabled".dimmed()
    );

    if total == 0 {
        println!(
            " {} {}",
            "⚠".yellow(),
            "No generators match criteria".italic().dimmed()
        );
    } else if enabled == 0 && no_filter {
        println!(
            " {} {}",
            "󰚔".yellow(),
            "Tip: use `iris gen enable <name>` to start".dimmed()
        );
    }
}
