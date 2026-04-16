use clap::Parser;
use colored::Colorize;
use iris::{cli::Cli, commands, core::IrisContext, ui::Logger};

fn main() {
    let cli: Cli = Cli::parse();
    let logger: Logger = Logger::new(cli.quiet);

    if let Err(err) = run(cli, logger) {
        eprintln!(
            "\n{} {} {}",
            "✘".red().bold(),
            "Error:".red().bold(),
            format!("{:#}", err).white()
        );
        std::process::exit(1);
    }
}

fn run(cli: Cli, logger: Logger) -> anyhow::Result<()> {
    let mut ctx: IrisContext = IrisContext::new(logger)?;
    commands::handle(cli.command, &mut ctx)?;
    Ok(())
}
