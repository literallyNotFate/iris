use crate::{
    commands::apply_theme,
    core::{IrisContext, ThemeOrchestrator},
    guards::CursorGuard,
    models::Theme,
    utils,
};
use colored::Colorize;
use dialoguer::console::Term;
use notify_debouncer_mini::{new_debouncer, notify::*};
use std::{
    fs,
    path::PathBuf,
    sync::mpsc::{RecvTimeoutError, channel},
    thread::sleep,
    time::{Duration, Instant},
};

/// Handle application watch command
pub fn exec(interval_ms: u64, ctx: &mut IrisContext) -> anyhow::Result<()> {
    let (tx, rx) = channel();
    let mut debouncer = new_debouncer(Duration::from_millis(interval_ms), tx)?;

    let cache_path: PathBuf = ctx.paths.current_theme.clone();
    debouncer
        .watcher()
        .watch(&cache_path, RecursiveMode::NonRecursive)?;

    let (exit_tx, exit_rx) = channel();
    ctrlc::set_handler(move || {
        let _ = exit_tx.send(());
    })?;

    let term: Term = Term::stdout();
    let _cursor_guard = CursorGuard::new(&term);

    render_watch_ui(&cache_path);

    loop {
        if exit_rx.try_recv().is_ok() {
            let _ = term.show_cursor();
            println!(
                "\n {}  {}\n",
                "󰈆".yellow().bold(),
                "Watcher stopped.".yellow()
            );
            break;
        }

        match rx.recv_timeout(Duration::from_millis(200)) {
            Ok(Ok(_)) => handle_change(&cache_path, ctx)?,
            Ok(Err(e)) => eprintln!(" {}  Watcher error: {:?}", "󰅙".red(), e),
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => anyhow::bail!("Watcher disconnected"),
        }
    }

    Ok(())
}

fn handle_change(path: &PathBuf, ctx: &mut IrisContext) -> anyhow::Result<()> {
    sleep(Duration::from_millis(10));

    let content: String = fs::read_to_string(path)?;
    let theme_name: String = content.trim().to_string();

    if theme_name.is_empty() || theme_name == ctx.state.current_theme {
        return Ok(());
    }

    println!(
        " {}  {} {}",
        "󱐋".yellow().bold(),
        "Change detected!".bold(),
        "Re-applying...".dimmed()
    );

    let start = Instant::now();
    let orchestrator: ThemeOrchestrator = ThemeOrchestrator::new(&ctx.paths, &ctx.log);
    let theme_obj: Theme = orchestrator.load_theme(&theme_name, false, true, &ctx.state)?;

    apply_theme(&theme_obj, ctx)?;
    render_watch_ui(path);

    println!(
        " {}  {} {} {} {}",
        "✓".green().bold(),
        "Theme".green(),
        theme_obj.name.cyan().bold(),
        "applied in".green(),
        format!("{:.2?}", start.elapsed()).white().bold()
    );

    Ok(())
}

fn render_watch_ui(path: &PathBuf) {
    let term = Term::stdout();
    let _ = term.clear_screen();

    println!("\n\n {}  {}", "󰈈".blue().bold(), "Iris Watch Mode".bold());
    println!(
        " {}  Watching: {}",
        "󰈚".dimmed(),
        utils::pretty_path(path).dimmed()
    );
    println!(" {}  {}", "󰜺".red(), "Press Ctrl+C to exit".dimmed());
    println!();
}
