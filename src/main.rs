use clap::Parser;
use colored::Colorize;
use iris::{
    cli::{Cli, Commands},
    config, generators, models, render,
};
use std::fs;

fn main() {
    config::setup_folders();
    let cli = Cli::parse();

    match &cli.command {
        Commands::List => {
            let themes_dir = config::get_base_path().join("themes");
            println!("{}", "Available Themes:".bold().bright_blue());
            if let Ok(entries) = fs::read_dir(themes_dir) {
                for entry in entries.flatten() {
                    let name = entry
                        .path()
                        .file_stem()
                        .unwrap()
                        .to_string_lossy()
                        .to_string();
                    println!("  {} {}", "→".bright_black(), name.green());
                }
            }
        }
        Commands::Switch { name } => {
            let theme_path = config::get_base_path()
                .join("themes")
                .join(format!("{}.toml", name));

            if let Ok(content) = fs::read_to_string(theme_path) {
                let theme: models::Theme = toml::from_str(&content).expect("Invalid TOML format");

                generators::ghostty::apply(&theme);

                config::save_state(name);
                render::display_palette(&theme);

                println!(
                    "{} Theme '{}' is now active.",
                    "Done!".bold().green(),
                    name.yellow()
                );
                println!("Now you can {} {}", "reload".bold().red(), "the configs!")
            } else {
                eprintln!("{} Theme '{}' not found.", "Error:".bold().red(), name);
            }
        }
        Commands::Status => {
            let state_path = config::get_base_path().join("state.json");
            let theme_name = fs::read_to_string(state_path)
                .ok()
                .and_then(|c| serde_json::from_str::<models::UIState>(&c).ok())
                .map(|s| s.current_theme)
                .unwrap_or_else(|| "none".to_string());

            println!("Current active theme: {}", theme_name.bold().cyan());
        }
    }
}
