use crate::{
    cli::ConfigAction,
    core::{Client, IrisContext},
    models::PluginManager,
};
use colored::*;

/// Handle application config command
pub fn exec(action: ConfigAction, ctx: &mut IrisContext) -> anyhow::Result<()> {
    match action {
        ConfigAction::Nvim { manager, detect } => {
            render_config_header("Neovim Configuration", "⚙");

            let selected: PluginManager = Client::choose(&ctx.paths, manager, detect, &ctx.log)?;
            Client::validate(&ctx.paths, &selected)?;

            let count: usize = Client::count_plugins(&ctx.paths, &selected);
            if count > 0 {
                println!(
                    "{} found {} {} {}",
                    "└──".dimmed(),
                    selected,
                    format!("({} plugins)", count).dimmed(),
                    "✓".green()
                );
            }

            ctx.state.manager = selected;
        }

        ConfigAction::Fallback { name } => {
            render_config_header("Fallback Configuration", "⚙");
            let theme: String = name.to_lowercase();
            ctx.validate_theme_exists(&theme)?;

            ctx.log.info(&format!(
                "Selecting {} as a fallback...",
                theme.to_string().magenta().bold()
            ));

            println!(
                "{} {} applied! {}",
                "└──".dimmed(),
                theme.magenta().bold(),
                "✓".green()
            );

            ctx.state.fallback_theme = theme;
        }
    }

    println!();

    ctx.log.action("Saved configuration to state.json\n", || {
        ctx.state.save_to(&ctx.paths.state_file)
    })?;

    println!();
    Ok(())
}

/// Helper function to render header for config
fn render_config_header(title: &str, icon: &str) {
    println!("\n{}  {}\n", icon.bright_yellow().bold(), title.bold());
}
