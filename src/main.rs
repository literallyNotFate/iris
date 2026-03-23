use anyhow::Result;
use clap::Parser;
use iris::{cli::Cli, commands, core::IrisContext};

fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut ctx: IrisContext = IrisContext::new()?;

    commands::handle(cli.command, &mut ctx)?;

    Ok(())
}
