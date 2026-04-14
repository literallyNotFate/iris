use crate::{core::IrisContext, utils};
use anyhow::{Context, Result};
use colored::Colorize;
use notify_debouncer_mini::{new_debouncer, notify::*};
use std::{
    fs,
    io::{self, Write},
    sync::mpsc::channel,
    time::{Duration, Instant},
};

/// Handle application watch command
pub fn exec(interval_ms: u64, ctx: &mut IrisContext) -> Result<()> {
    let (tx, rx) = channel();

    let mut debouncer = new_debouncer(Duration::from_millis(interval_ms), tx)
        .context("Failed to create watcher")?;
    let cache_path = ctx.paths.cache.join("current_theme");

    debouncer
        .watcher()
        .watch(&cache_path, RecursiveMode::NonRecursive)
        .context("Failed to start watching")?;

    let print_header = |path: &str| {
        print!("\x1B[2J\x1B[1;1H");
        println!(" {} {}", "󰈈".blue().bold(), "Iris Watch Mode".bold());
        println!(" {} Watching: {}", "󰈚".dimmed(), path.dimmed());
        println!(" {} {}", "󰜺".red(), "Press Ctrl+C to exit".dimmed());
        println!();
        io::stdout().flush().unwrap();
    };

    print_header(&utils::pretty_path(&cache_path));

    let (exit_tx, exit_rx) = channel();
    ctrlc::set_handler(move || {
        let _ = exit_tx.send(());
    })
    .expect("Error setting Ctrl-C handler");

    loop {
        if let Ok(_) = exit_rx.try_recv() {
            println!("\n {} {}", "󰈆".yellow().bold(), "Watcher stopped.".yellow());
            break;
        }

        if let Ok(result) = rx.recv_timeout(Duration::from_millis(500)) {
            match result {
                Ok(_) => {
                    let content = fs::read_to_string(&cache_path).unwrap_or_default();
                    let theme = content.trim().to_string();

                    if theme.is_empty() || theme == ctx.state.current_theme {
                        continue;
                    }

                    println!(
                        " {} {} {}",
                        "󱐋".yellow().bold(),
                        "Change detected!".bold(),
                        "Re-applying...".dimmed()
                    );

                    let original_quiet: bool = ctx.log.quiet;
                    ctx.log.quiet = true;
                    let start = Instant::now();

                    let res = super::apply_theme(&theme, ctx);

                    ctx.log.quiet = original_quiet;

                    print_header(&cache_path.display().to_string());

                    match res {
                        Ok(_) => {
                            println!(
                                " {} {} {} {} {}",
                                "󰄬".green().bold(),
                                "Theme".green(),
                                theme.cyan().bold(),
                                "applied in".green(),
                                format!("{:.2?}", start.elapsed()).white().bold()
                            );
                        }
                        Err(e) => {
                            eprintln!(" {} {} {}", "󰅙".red(), "Error:".red().bold(), e);
                        }
                    }
                }
                Err(e) => eprintln!("Watcher error: {:?}", e),
            }
        }
    }

    Ok(())
}
