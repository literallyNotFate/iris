use clap::Parser;
use colored::Colorize;
use iris::{cli::Cli, commands, core::IrisContext, log::Logger};

fn main() {
    let cli: Cli = Cli::parse();
    let reporter: Logger = if cli.quiet {
        Logger::minimal()
    } else {
        Logger::new()
    };

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

fn run(cli: Cli, reporter: Logger) -> anyhow::Result<()> {
    let mut ctx: IrisContext = IrisContext::new(reporter)?;
    commands::handle(cli.command, &mut ctx)?;
    Ok(())
}
