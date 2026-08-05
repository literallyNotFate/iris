/// Handle application diff command
pub fn exec(generator: String, ctx: &crate::core::IrisContext) -> anyhow::Result<()> {
    use colored::*;

    let theme = &ctx.state.theme.current_theme;
    let generator = ctx.resolve_generator(&generator)?;

    println!(
        "\n{}  Diff for: {}\n",
        "󰊢".green().bold(),
        generator.name().cyan().bold()
    );

    match generator.diff(&ctx.paths, theme)? {
        Some(diff_output) => {
            print!("{}", diff_output);
        }
        None => {
            println!("  {}", "✓ No differences. Config is in sync.".green());
        }
    }

    println!();
    Ok(())
}
