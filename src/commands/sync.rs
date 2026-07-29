/// Handle application sync command
pub fn exec(force: bool, ctx: &mut crate::core::IrisContext) -> anyhow::Result<()> {
    use colored::*;
    println!();

    let main_activity = ctx
        .log
        .step_with_icon("󰓦".cyan().bold(), "Synchronizing state...", true);

    let service = crate::service::ThemeService::new(&ctx.paths, &ctx.log);
    let (nvim_theme, is_synced) = service.sync_status(&ctx.state);
    let is_dirty: bool = ctx.is_any_config_broken();

    if is_synced && !is_dirty && !force {
        main_activity.done_with("Everything is already in sync!");
        if !ctx.log.is_detailed() {
            println!();
        }

        return Ok(());
    }

    if is_dirty && is_synced && !force {
        main_activity.warn("Found broken configs, restoring...");
    }

    let theme_obj = service.load_theme(&nvim_theme, force, true, &ctx.state)?;
    crate::commands::apply_theme(&theme_obj, ctx)?;

    main_activity.done_with("All apps are now in sync!");
    Ok(())
}
