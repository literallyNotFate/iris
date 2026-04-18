use crate::{cli::ConfigAction, core::IrisContext, models::NvimStrategy};
use colored::*;

/// Handle application config command
pub fn exec(action: ConfigAction, ctx: &mut IrisContext) -> anyhow::Result<()> {
    let ConfigAction::Nvim { strategy, detect } = action;

    println!(
        "\n{}  {}\n",
        "⚙".bright_yellow().bold(),
        "Neovim Configuration".bold()
    );

    let selected: NvimStrategy = NvimStrategy::choose(strategy, detect, ctx)?;
    selected.validate()?;

    let count: usize = selected.count_plugins();
    if count > 0 {
        println!(
            "      {}  Status: {} plugins found",
            "󰄬".green().bold(),
            count.to_string().cyan().bold()
        );
    }

    ctx.state.nvim = selected;
    println!();
    let mut s = ctx.log.step("Updating state.json", 1);
    ctx.state.save_to(&ctx.paths.state_file)?;
    s.done(true);

    println!(
        "\n{}  Iris is now synced with {}",
        "✔".green().bold(),
        ctx.state.nvim
    );

    Ok(())
}
