
/// Handle application sync command
pub fn exec(force: bool, ctx: &mut crate::core::IrisContext) -> anyhow::Result<()> {
    use colored::*;
    println!();

    let main_task =
        ctx.log
            .step_with_icon("󰓦".cyan().bold(), "Synchronizing system state...", true);

    let service = crate::service::ThemeService::new(&ctx.paths, &ctx.log);
    let (nvim_theme, is_synced) = service.sync_status(&ctx.state);
    let is_dirty: bool = ctx.is_any_config_broken();

    if is_synced && !is_dirty && !force {
        main_task.done_with("Everything is already in sync");
        if !ctx.log.is_detailed() {
            println!();
        }

        return Ok(());
    }

    if is_dirty && is_synced && !force {
        main_task.warn("Found broken configs, restoring...");
    }

    let theme_obj = service.load_theme(&nvim_theme, force, true, &ctx.state)?;
    crate::commands::apply_theme(&theme_obj, ctx)?;

    main_task.done_with("All apps are now in sync!");
    Ok(())
}
