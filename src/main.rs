use anyhow::{Context as _, Result};
use clap::Parser;
use colored::*;
use iris::{
    cli::{Cli, Commands},
    context::AppContext,
    models::Palette,
    modules, render,
    setup::Setup,
};

fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut ctx: AppContext = AppContext::new()?;

    match &cli.command {
        Commands::Init => {
            println!("\n{}", " Initializing Iris ".on_blue().white().bold());
            Setup::run(&ctx)?;

            let theme: String = Palette::current()?;
            let palette: Palette = Palette::fetch(&theme)?;

            modules::apply_all(&palette, &ctx)?;
            println!("{}", " Iris is ready to go!".green());
        }

        Commands::Switch { name } => {
            let theme: String = match name {
                Some(n) => n.clone(),
                None => Palette::current()?,
            };

            let palette: Palette = Palette::fetch(&theme)
                .with_context(|| format!("Failed to fetch colors for '{}'", theme))?;

            modules::apply_all(&palette, &ctx)?;
            ctx.update(&theme)?;
        }

        Commands::Status => {
            println!("\n{}", " Iris Status ".on_cyan().black().bold());
            let current = &ctx.state.current_theme;

            println!("  Active theme: {}", current.bold().cyan());
            println!(
                "  Enabled apps: {}",
                ctx.state.enabled_generators.join(", ").yellow()
            );
            println!(
                "  Config path:  {}",
                ctx.base_path.display().to_string().bright_black()
            );

            if let Ok(palette) = Palette::fetch(current) {
                if let Ok(nvim_theme) = Palette::current() {
                    if nvim_theme != *current {
                        println!(
                            "\n  {} {}",
                            "⚠".yellow(),
                            "Out of sync with active Neovim session".yellow()
                        );
                        println!("    Current in Nvim: {}", nvim_theme.bright_yellow());
                        println!("    Run `iris switch` to sync.");
                    }
                }

                render::display_palette(&palette, current);
            } else {
                println!("\n  {}", "✕ Could not load palette for preview".red());
            }
        }
    }

    Ok(())
}
