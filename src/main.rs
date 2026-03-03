use anyhow::{Context as _, Result};
use clap::Parser;
use colored::*;
use iris::{
    cli::{Cli, Commands},
    context::AppContext,
    generators,
    models::Theme,
    render,
    setup::Setup,
};
use std::fs;

fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut ctx: AppContext = AppContext::new()?;

    match &cli.command {
        Commands::Init => {
            println!("\n{}", " Initializing Iris ".on_blue().white().bold());
            Setup::run(&ctx)?;
            println!("{}", " Iris is ready to go!".green());
        }

        Commands::List => {
            println!(
                "\n{}",
                " Available Themes ".on_bright_black().white().bold()
            );

            let themes_dir = ctx.themes_dir();
            if let Ok(entries) = fs::read_dir(&themes_dir) {
                let mut entries: Vec<_> = entries.flatten().collect();
                entries.sort_by_key(|e| e.file_name());

                for entry in entries {
                    let name = entry
                        .path()
                        .file_stem()
                        .unwrap()
                        .to_string_lossy()
                        .to_string();

                    if name == ctx.state.current_theme {
                        println!(
                            "  {} {} {}",
                            "●".green(),
                            name.green().bold(),
                            "(active)".bright_black()
                        );
                    } else {
                        println!("  {} {}", "○".bright_black(), name.white());
                    }
                }
            }
            println!();
        }

        Commands::Switch { name } => {
            let theme = Theme::load_by_name(name, &ctx)
                .with_context(|| format!("Failed to load theme '{}'", name))?;

            generators::apply_all(&theme, &ctx)?;
            ctx.update_theme(name)?;
            render::display_palette(&theme);

            println!(
                "{} Theme '{}' is now active.",
                "Done!".bold().green(),
                name.yellow()
            );
        }

        Commands::Status => {
            println!("\n{}", " Iris Status ".on_cyan().black().bold());
            println!("  Active theme: {}", ctx.state.current_theme.bold().cyan());
            println!(
                "  Enabled apps: {}",
                ctx.state.enabled_generators.join(", ").yellow()
            );
            println!(
                "  Config path:  {}",
                ctx.base_path.display().to_string().bright_black()
            );
            println!();
        }
    }

    Ok(())
}
