use crate::{
    core::IrisContext,
    infra::{NeovimBridge, RESOURCES_DIR},
    log::Activity,
    models::PluginManager,
    service::ThemeService,
    utils,
};
use colored::Colorize;
use std::{fs, path::PathBuf};

/// Struct for initializing state of application
pub struct IrisSetup;

impl IrisSetup {
    pub fn run(ctx: &mut IrisContext) -> anyhow::Result<()> {
        if ctx.log.is_detailed() {
            eprintln!();
            eprintln!("{}  {}", "󰒓".purple().bold(), "Iris initialization".bold());
        }
        eprintln!();

        ctx.log
            .action("Prepared file structure.", || ctx.paths.ensure_dirs())?;
        if ctx.log.is_detailed() {
            eprintln!();
        }

        {
            let activity = ctx.log.step_with_icon(
                &"󰏘".red().bold(),
                "Initializing application state...",
                false,
            );
            Self::setup_initial_state(ctx, &activity)?;
            activity.done_with("System state initialized");
        }

        {
            let activity =
                ctx.log
                    .step_with_icon(&"󰒓".green().bold(), "Integrating with `nvim`...", true);
            Self::setup_nvim_automation(ctx, &activity)?;
            activity.done_with("`nvim` automation hook generated smoothly");
        }

        ctx.log
            .success("Iris is now fully configured and ready to go!");
        eprintln!();

        Ok(())
    }

    /// Initializes state
    fn setup_initial_state(ctx: &mut IrisContext, activity: &Activity) -> anyhow::Result<()> {
        if ctx.paths.state_file.exists() {
            activity.info("Found existing state.toml file, loading configuration...");
            return Ok(());
        }

        activity.info("Detecting `nvim` plugin manager...");
        let manager: PluginManager = NeovimBridge::detect(&ctx.paths);

        if !manager.is_default() {
            let count: usize = NeovimBridge::count(&ctx.paths, &manager);
            activity.info(&format!(
                "Found {} with {} plugins installed!",
                manager,
                count.to_string().yellow().bold()
            ));
        }

        ctx.state.nvim.manager = manager;
        activity.info("Detecting active `nvim` theme...");

        let service: ThemeService<'_> = ThemeService::new(&ctx.paths, &activity.log);
        let current_theme: String = service.current().unwrap_or_else(|_| "".to_string());

        activity.info("Scanning for compatible tools...");
        let installed = ctx.registry.installed();

        if installed.is_empty() {
            activity.info("No compatible tools found.");
        } else {
            for generator in installed.iter() {
                activity.info(&format!("Found: {}", generator.name().green().bold()));
                ctx.state.enable_generator(generator.name());
            }
        }

        ctx.state.set_theme(current_theme);
        ctx.save()?;

        activity.info(&format!(
            "Configuration persisted to {}.",
            utils::pretty_path(&ctx.paths.state_file).dimmed()
        ));
        Ok(())
    }

    /// Generates Neovim site plugin script for automatic synchronization
    fn setup_nvim_automation(ctx: &IrisContext, activity: &Activity) -> anyhow::Result<()> {
        let nvim_plugin_dir: PathBuf = ctx.paths.nvim_data_path().join("site/plugin");
        let plugin_file: PathBuf = nvim_plugin_dir.join("iris_sync.lua");

        let autoloader: &str = RESOURCES_DIR
            .get_file("lua/nvim_sync.lua")
            .expect("nvim_sync.lua must be included")
            .contents_utf8()
            .expect("File must be valid utf8");

        if plugin_file.exists() {
            if let Ok(existing_content) = fs::read_to_string(&plugin_file) {
                if existing_content == autoloader {
                    activity.info("`nvim` integration is already up to date!");
                    return Ok(());
                }
            }
        }

        activity.info("Writing `nvim` autoloader plugin...");
        fs::create_dir_all(&nvim_plugin_dir)?;
        fs::write(&plugin_file, autoloader)?;

        activity.info(&format!(
            "Automation persisted to {}.",
            crate::utils::pretty_path(&plugin_file).dimmed()
        ));

        Ok(())
    }

    /// Generates initializing script right into stdout for eval
    pub fn emit_zsh_hook(_ctx: &IrisContext) -> anyhow::Result<()> {
        let autocomplete: &str = RESOURCES_DIR
            .get_file("zsh/autocomplete.zsh")
            .expect("autocomplete.zsh must be included")
            .contents_utf8()
            .expect("File must be valid utf8");

        println!("{}", autocomplete.trim());
        Ok(())
    }
}

/// Unit-tests for setup
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_generate_zsh_hook_correctly() {
        let (_tmp, _ctx) = IrisContext::mock();

        let autocomplete = RESOURCES_DIR
            .get_file("zsh/autocomplete.zsh")
            .expect("autocomplete.zsh must be included")
            .contents_utf8()
            .unwrap();

        assert!(autocomplete.contains("_iris_completion"));
        assert!(autocomplete.contains("compdef _iris_completion iris"));
    }

    #[test]
    fn should_skip_initial_setup_if_exists() {
        let (_tmp, mut ctx) = IrisContext::mock();

        fs::write(
            &ctx.paths.state_file,
            r#"{"current_theme": "nord", "enabled_generators": []}"#,
        )
        .unwrap();

        let activity = ctx.log.step("Initial State Test", true);
        let result = IrisSetup::setup_initial_state(&mut ctx, &activity);

        assert!(result.is_ok());
    }

    #[test]
    fn should_handle_full_setup_logic() {
        let (tmp, mut ctx) = IrisContext::mock();
        let fake_home = tmp.path();

        temp_env::with_vars(
            [
                ("HOME", Some(fake_home.to_str().unwrap())),
                (
                    "XDG_CONFIG_HOME",
                    Some(fake_home.join(".config").to_str().unwrap()),
                ),
                (
                    "XDG_DATA_HOME",
                    Some(fake_home.join(".local/share").to_str().unwrap()),
                ),
            ],
            || {
                let result = IrisSetup::run(&mut ctx);
                assert!(result.is_ok(), "Setup run failed: {:?}", result.err());
                assert!(ctx.paths.config.exists(), "Config dir should be created");
                assert!(ctx.paths.cache.exists(), "Cache dir should be created");
                assert!(
                    ctx.paths.state_file.exists(),
                    "state file should be created"
                );

                let expected_lua_path =
                    ctx.paths.nvim_data_path().join("site/plugin/iris_sync.lua");
                assert!(
                    expected_lua_path.exists(),
                    "Neovim autoloader script should be generated at {:?}",
                    expected_lua_path
                );

                assert_eq!(ctx.state.nvim.manager, PluginManager::Default);
            },
        );
    }
}
