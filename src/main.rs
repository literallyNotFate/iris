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
            Status::step("Performing initial sync...", 0);

            let theme: String = Palette::current()?;
            let palette: Palette = Palette::fetch(&theme)?;

            modules::apply_all(&palette, &ctx)?;
            println!(
                "\n{}",
                " Iris is now fully configured and ready to go!"
                    .green()
                    .bold()
            );
        }

        Commands::Switch { name } => {
            let theme: String = match name {
                Some(n) => n.clone(),
                None => {
                    Status::step("Detecting theme from Neovim...", 0);
                    Palette::current()?
                }
            };

            println!(
                "\n{}\n",
                format!("Switching to {}...", theme).bold().yellow()
            );
            let palette: Palette = Palette::fetch(&theme)
                .with_context(|| format!("Failed to fetch colors for '{}'", theme))?;

            modules::apply_all(&palette, &ctx)?;

            Status::step("Updating local state...", 1);
            ctx.update(&theme)?;

            println!();
            Status::success(&format!("Theme {} applied to all apps.", theme.cyan()), 0);
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
