/// Handle application current command
pub fn exec(ctx: &crate::core::IrisContext) -> anyhow::Result<()> {
    use colored::*;

    let active_theme: String = if ctx.paths.current_theme.exists() {
        let content: String = std::fs::read_to_string(&ctx.paths.current_theme)?;
        let trimmed: String = content.trim().to_string();

        if !trimmed.is_empty() {
            trimmed
        } else {
            ctx.state.theme.current_theme.clone()
        }
    } else {
        ctx.state.theme.current_theme.clone()
    };

    if ctx.log.is_detailed() {
        println!("\nActive theme: {}\n", active_theme.cyan().bold());
    } else {
        println!("{}", active_theme);
    }

    Ok(())
}
