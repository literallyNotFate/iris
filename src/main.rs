use clap::Parser;
use colored::Colorize;
use iris::{cli::Cli, commands, core::IrisContext, log::Reporter};

fn main() {
    let cli: Cli = Cli::parse();
    let reporter: Reporter = if cli.quiet {
        Reporter::quiet()
    } else {
        Reporter::new()
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

fn run(cli: Cli, reporter: Reporter) -> anyhow::Result<()> {
    let mut ctx: IrisContext = IrisContext::new(reporter)?;
    commands::handle(cli.command, &mut ctx)?;
    Ok(())
}
