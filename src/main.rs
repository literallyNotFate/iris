use anyhow::{Context as _, Result};
use clap::Parser;
use colored::*;
use iris::{
    cli::{Cli, Commands},
    context::AppContext,
    models::Palette,
    modules, render,
    setup::Setup,
    status::Status,
};

fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut ctx: AppContext = AppContext::new()?;

    match &cli.command {
        Commands::Init => {
            Setup::run(&ctx)?;

            println!();
            let sync_task = Status::step("Performing initial sync...", 0);

            let theme: String = Palette::current()?;
            let palette: Palette = Palette::fetch(&theme)?;

            modules::apply_all(&palette, &ctx)?;

            sync_task.done(Some("Initial sync complete."));
            println!(
                "\n{}",
                " Iris is now fully configured and ready to go!"
                    .green()
                    .bold()
            );
        }

        Commands::Switch { name } => {
            let (theme, is_manual) = match name {
                Some(n) => (n.clone(), true),
                None => {
                    let task = Status::step("Detecting theme from Neovim...", 0);
                    let t = Palette::current()?;
                    task.done(Some("Name not specified, returning last used"));
                    (t, false)
                }
            };

            if is_manual && !Palette::exists(&theme) {
                Status::error(
                    &format!("Theme '{}' not found in Neovim.", theme.red().bold()),
                    0,
                );
                println!(
                    "  {} Run `:colorscheme <Tab>` in Neovim to see available themes.",
                    "Tip:".blue()
                );
                return Ok(());
            }

            println!(
                "\n{}\n",
                format!("Switching to {}...", theme).bold().yellow()
            );
            let switch_task = Status::step(&format!("Applying {} palette...", theme.cyan()), 0);
            let palette: Palette = Palette::fetch(&theme)
                .with_context(|| format!("Failed to fetch colors for '{}'", theme))?;

            modules::apply_all(&palette, &ctx)?;

            let state_task = Status::step("Updating local state...", 1);
            ctx.update(&theme)?;
            state_task.done(Some("Local state updated!"));

            println!();
            switch_task.done(Some(&format!(
                "Theme {} applied to all apps.",
                theme.cyan()
            )));
        }

        Commands::Status => {
            println!("\n{}\n", "Iris System Status".purple());
            let current = &ctx.state.current_theme;

            println!("  {} Active theme: {}", "●".blue(), current.bold().blue());
            println!(
                "  {} Enabled apps: {}",
                "●".yellow(),
                ctx.state.enabled_generators.join(", ").white()
            );
            println!(
                "  {} Config path:  {}",
                "●".white(),
                ctx.base_path.display().to_string().bright_black()
            );

            if let Ok(nvim_theme) = Palette::current() {
                if nvim_theme.to_lowercase() != current.to_lowercase() {
                    println!(
                        "\n  {} {}",
                        "⚠".yellow(),
                        "Out of sync with Neovim".yellow().bold()
                    );
                    println!("    Neovim: {}", nvim_theme.bright_yellow());
                    println!("    Iris:   {}", current.dimmed());
                } else {
                    println!("\n  {} {}", "✔".green(), "Synchronized with Neovim".green());
                }
            }

            if let Ok(palette) = Palette::fetch(current) {
                render::display_palette(&palette, current);
            }
        }
    }

    Ok(())
}
