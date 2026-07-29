use clap::Parser;
use iris::{cli::Cli, commands, core::IrisContext, log::Logger};

/// Main entry point
fn main() {
    use colored::Colorize;
    use std::io::{IsTerminal, stdout};

    let cli: Cli = Cli::parse();
    let mut reporter: Logger = if cli.quiet {
        Logger::minimal()
    } else {
        Logger::new()
    };

    if !stdout().is_terminal() {
        reporter.verbosity = iris::log::LoggingVerbosity::Silent;
    }

    if let Err(err) = run(cli, reporter) {
        eprintln!(
            "\n{} {} {}",
            "✘".red().bold(),
            "Execution failed:".red().bold(),
            err.to_string().white().bold()
        );

        let mut cause = err.source();
        if cause.is_some() {
            eprintln!("\n{}", "Caused by:".dimmed().underline());
            while let Some(src) = cause {
                eprintln!("  {} {}", "•".dimmed(), src.to_string().dimmed());
                cause = src.source();
            }
        }

        eprintln!();
        std::process::exit(1);
    }
}

/// Run application
fn run(cli: Cli, reporter: Logger) -> anyhow::Result<()> {
    let mut ctx: IrisContext = IrisContext::new(reporter)?;
    commands::handle(cli.command, &mut ctx)?;
    Ok(())
}
