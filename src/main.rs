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
            "\n{} {} {}\n",
            "✘".red().bold(),
            "Error:".red().bold(),
            format!("{:#}", err).white()
        );

        std::process::exit(1);
    }
}

/// Run application
fn run(cli: Cli, reporter: Logger) -> anyhow::Result<()> {
    let mut ctx: IrisContext = IrisContext::new(reporter)?;
    commands::handle(cli.command, &mut ctx)?;
    Ok(())
}
