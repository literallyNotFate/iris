use crate::{
    cli::ConfigAction,
    core::{IrisContext, ThemeOrchestrator},
    models::PluginManager,
    utils,
};
use colored::*;

/// Handle application config command
pub fn exec(action: ConfigAction, ctx: &mut IrisContext) -> anyhow::Result<()> {
    match action {
        ConfigAction::Nvim { manager, detect } => {
            render_config_header("Neovim Configuration", "⚙");

            let orchestrator: ThemeOrchestrator = ThemeOrchestrator::new(&ctx.paths, &ctx.log);
            let selected: PluginManager = orchestrator.choose_manager(manager, detect)?;
            ctx.state.manager = selected;
        }

        ConfigAction::Fallback { name } => {
            render_config_header("Fallback Configuration", "⚙");

            let theme: String = utils::capitalize(name.trim());
            let orchestrator: ThemeOrchestrator = ThemeOrchestrator::new(&ctx.paths, &ctx.log);
            if !orchestrator.theme_exists(&theme, &ctx.state) {
                anyhow::bail!(
                    "Theme `{}` does not exist in Neovim or cache.",
                    theme.cyan().bold()
                );
            }

            ctx.log.info(&format!(
                "Selecting {} as a fallback...",
                theme.magenta().bold()
            ));

            println!(
                "{} {} applied! {}",
                "└──".dimmed(),
                theme.magenta().bold(),
                "✓".green()
            );

            ctx.state.fallback_theme = theme.to_lowercase();
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
