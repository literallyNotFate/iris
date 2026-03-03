use clap::Parser;
use colored::*;
use iris::{
    cli::{Cli, Commands},
    config, generators, models, render,
};
use std::fs;

fn main() {
    let cli: Cli = Cli::parse();

    match &cli.command {
        Commands::Init => {
            config::init_project();
        }

        Commands::List => {
            let state = config::get_state();
            let themes_dir = config::get_base_path().join("themes");

            println!(
                "\n{}",
                " Available Themes ".on_bright_black().white().bold()
            );

            if let Ok(entries) = fs::read_dir(themes_dir) {
                let mut entries: Vec<_> = entries.flatten().collect();
                entries.sort_by_key(|e| e.file_name());

                for entry in entries {
                    let name = entry
                        .path()
                        .file_stem()
                        .unwrap()
                        .to_string_lossy()
                        .to_string();
                    if name == state.current_theme {
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
            let theme_path = config::get_base_path()
                .join("themes")
                .join(format!("{}.toml", name));

            if let Ok(content) = fs::read_to_string(theme_path) {
                let theme: models::Theme = toml::from_str(&content).expect("Invalid TOML format");
                let state = config::get_state();

                generators::apply_enabled(&theme, &state.enabled_generators);

                config::save_state(name);
                render::display_palette(&theme);

                println!(
                    "{} Theme '{}' is now active across all enabled apps.",
                    "Done!".bold().green(),
                    name.yellow()
                );
            } else {
                eprintln!(
                    "{} Theme '{}' not found in themes folder.",
                    "Error:".bold().red(),
                    name
                );
            }
        }

        Commands::Status => {
            let state = config::get_state();
            println!("\n{}", " Iris Status ".on_cyan().black().bold());
            println!("  Active theme: {}", state.current_theme.bold().cyan());
            println!(
                "  Enabled apps: {}",
                state.enabled_generators.join(", ").yellow()
            );
            println!(
                "  Config path:  {}",
                config::get_base_path().display().to_string().bright_black()
            );
            println!();
        }
    }
}
