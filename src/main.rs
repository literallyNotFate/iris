use anyhow::Result;
use clap::Parser;
use iris::{cli::Cli, commands, core::IrisContext, ui::Logger};

fn main() -> Result<()> {
    let cli: Cli = Cli::parse();
    let logger: Logger = Logger::new(cli.quiet);
    let mut ctx: IrisContext = IrisContext::new(logger)?;

    commands::handle(cli.command, &mut ctx)?;

    Ok(())
}
