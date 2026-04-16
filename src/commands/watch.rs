use crate::{core::IrisContext, utils};
use anyhow::{Context, Result};
use colored::Colorize;
use notify_debouncer_mini::{new_debouncer, notify::*};
use std::{
    fs,
    io::{self, Write},
    path::PathBuf,
    sync::mpsc::{RecvTimeoutError, channel},
    time::{Duration, Instant},
};

/// Handle application watch command
pub fn exec(interval_ms: u64, ctx: &mut IrisContext) -> Result<()> {
    let (tx, rx) = channel();

    let mut debouncer = new_debouncer(Duration::from_millis(interval_ms), tx)
        .context("Failed to initialize file watcher")?;
    let cache_path: PathBuf = ctx.paths.cache.join("current_theme");

    debouncer
        .watcher()
        .watch(&cache_path, RecursiveMode::NonRecursive)
        .with_context(|| format!("Failed to watch path: {}", cache_path.display()))?;

    let print_header = |path: &str| {
        print!("\x1B[2J\x1B[1;1H");
        println!(" {} {}", "󰈈".blue().bold(), "Iris Watch Mode".bold());
        println!(" {} Watching: {}", "󰈚".dimmed(), path.dimmed());
        println!(" {} {}", "󰜺".red(), "Press Ctrl+C to exit".dimmed());
        println!();
        let _ = io::stdout().flush();
    };

    print_header(&utils::pretty_path(&cache_path));

    let (exit_tx, exit_rx) = channel();
    ctrlc::set_handler(move || {
        let _ = exit_tx.send(());
    })
    .context("Error setting Ctrl-C handler")?;

    loop {
        if exit_rx.try_recv().is_ok() {
            println!("\n {} {}", "󰈆".yellow().bold(), "Watcher stopped.".yellow());
            break;
        }

        match rx.recv_timeout(Duration::from_millis(200)) {
            Ok(result) => match result {
                Ok(_) => {
                    let content: String = fs::read_to_string(&cache_path)
                        .with_context(|| format!("Could not read {}", cache_path.display()))?;

                    let theme: String = content.trim().to_string();
                    if theme.is_empty() || theme == ctx.state.current_theme {
                        continue;
                    }

                    println!(
                        " {}  {} {}",
                        "󱐋".yellow().bold(),
                        "Change detected!".bold(),
                        "Re-applying...".dimmed()
                    );

                    let start = Instant::now();
                    let original_quiet: bool = ctx.log.quiet;
                    ctx.log.quiet = true;

                    let res = super::apply_theme(&theme, ctx);
                    ctx.log.quiet = original_quiet;

                    print_header(&utils::pretty_path(&cache_path));

                    match res {
                        Ok(_) => {
                            println!(
                                " {}  {} {} {} {}",
                                "󰄬".green().bold(),
                                "Theme".green(),
                                theme.cyan().bold(),
                                "applied in".green(),
                                format!("{:.2?}", start.elapsed()).white().bold()
                            );
                        }
                        Err(e) => {
                            eprintln!(
                                " {}  {} {:?}",
                                "󰅙".red(),
                                "Application error:".red().bold(),
                                e
                            );
                        }
                    }
                }
                Err(e) => {
                    eprintln!(" {}  Watcher error: {:?}", "󰅙".red(), e);
                }
            },
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => anyhow::bail!("Watcher channel disconnected"),
        }
    }

    Ok(())
}
