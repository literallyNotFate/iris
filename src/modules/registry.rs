use crate::{
    core::{IrisPaths, Templater},
    log::Reporter,
    models::{Palette, State},
    modules::{Generator, GeneratorType},
};
use anyhow::{Context, Result};
use colored::Colorize;
use std::{collections::BTreeSet, sync::Arc};

/// The list of all generators
#[derive(Default, Clone)]
pub struct GeneratorRegistry {
    pub generators: Vec<Arc<dyn Generator>>,
}

impl GeneratorRegistry {
    /// Creates registy and appends all generators/modules
    pub fn new() -> Self {
        use crate::modules::{multiplexer, prompts, system, terminals, tools};
        let mut generators: Vec<Arc<dyn Generator>> = Vec::new();

        generators.extend(terminals::get_all());
        generators.extend(prompts::get_all());
        generators.extend(tools::get_all());
        generators.extend(system::get_all());
        generators.extend(multiplexer::get_all());

        generators.sort_by(|a, b| {
            a.generator_type()
                .cmp(&b.generator_type())
                .then(a.name().cmp(b.name()))
        });

        Self { generators }
    }

    /// Access to all generators
    pub fn all(&self) -> Vec<&dyn Generator> {
        self.generators.iter().map(|b| b.as_ref()).collect()
    }

    /// List of all supported generators names (for MultiSelect)
    pub fn names(&self) -> Vec<String> {
        self.generators
            .iter()
            .map(|g| g.name().to_string())
            .collect()
    }

    /// List of all unique type that are enabled in registry
    pub fn types(&self) -> BTreeSet<GeneratorType> {
        self.generators.iter().map(|g| g.generator_type()).collect()
    }

    /// Get generator by name
    pub fn get(&self, name: &str) -> Option<&dyn Generator> {
        self.generators
            .iter()
            .find(|g| g.name() == name)
            .map(|b| b.as_ref())
    }

    /// Get generators by type
    pub fn by_type(&self, g_type: GeneratorType) -> Vec<&dyn Generator> {
        self.generators
            .iter()
            .filter(|g| g.generator_type() == g_type)
            .map(|b| b.as_ref())
            .collect()
    }

    /// Get list of all installed tools
    pub fn installed(&self) -> Vec<&dyn Generator> {
        self.generators
            .iter()
            .filter(|g| g.is_installed())
            .map(|b| b.as_ref())
            .collect()
    }

    /// Check whether this generator is installed in system
    pub fn is_installed(&self, name: &str) -> bool {
        self.get(name).map(|g| g.is_installed()).unwrap_or(false)
    }

    /// Checks whether this generator exists
    pub fn exists(&self, name: &str) -> bool {
        self.generators.iter().any(|g| g.name() == name)
    }

    /// Discover unenabled generators if they are installed
    pub fn discover_unenabled(&self, state: &State) -> Vec<&dyn Generator> {
        self.generators
            .iter()
            .filter(|g| g.is_installed() && !state.is_enabled(g.name()))
            .map(|b| b.as_ref())
            .collect()
    }

    /// Get all active generators (returns joined string of array of names)
    pub fn active(&self, state: &State) -> String {
        self.generators
            .iter()
            .filter(|g| state.is_enabled(g.name()))
            .map(|n| n.name().cyan().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl GeneratorRegistry {
    /// Apply themes to available programs (enabled generators)
    pub fn apply_all(
        &self,
        palette: &Palette,
        state: &State,
        paths: &IrisPaths,
        templater: &Templater,
        log: &Reporter,
    ) -> Result<()> {
        let to_apply: Vec<&dyn Generator> = self
            .generators
            .iter()
            .filter(|g| state.is_enabled(g.name()) && g.is_installed())
            .map(|b| b.as_ref())
            .collect();

        if to_apply.is_empty() {
            log.warn("No targets enabled or installed");
            return Ok(());
        }

        let total = to_apply.len();
        let root = log.step_with_icon(
            "󰛓".blue().bold(),
            &format!("Updating {} targets...", total.to_string().blue().bold()),
            true,
        );

        for (i, generator) in to_apply.iter().enumerate() {
            let is_last: bool = i == total - 1;
            let generator_color = generator.generator_type().color();
            let generator_icon = generator
                .generator_type()
                .icon()
                .color(generator_color)
                .bold();

            let mut task = root.log.step_with_icon(
                generator_icon,
                &format!("{}", generator.name().color(generator_color).bold()),
                is_last,
            );

            generator
                .apply(palette, paths, templater, &mut task)
                .with_context(|| {
                    format!(
                        "Failed to apply theme to `{}`",
                        generator.name().bold().green()
                    )
                })?;

            task.done_with(&format!("{} updated!", generator.name().cyan()));
        }

        root.done_with(&format!(
            "All {} targets were updated!",
            total.to_string().blue()
        ));

        Ok(())
    }
}

/// Unit-tests for generator registry
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        core::{IrisPaths, Templater},
        log::Task,
        models::HealthStatus,
    };

    // Mock generator and trait implementation
    struct MockGenerator {
        name: &'static str,
        g_type: GeneratorType,
        installed: bool,
    }

    impl Generator for MockGenerator {
        fn name(&self) -> &str {
            self.name
        }

        fn generator_type(&self) -> GeneratorType {
            self.g_type
        }

        fn target_file_name(&self, theme: &str) -> String {
            format!("{}.conf", theme)
        }

        fn is_installed(&self) -> bool {
            self.installed
        }

        fn apply(
            &self,
            _: &Palette,
            _: &IrisPaths,
            _: &Templater,
            _: &mut Task,
        ) -> anyhow::Result<()> {
            Ok(())
        }

        fn build_render_context(&self, _: &Palette) -> tera::Context {
            tera::Context::new()
        }

        fn fix(
            &self,
            _: &HealthStatus,
            _: &Palette,
            _: &IrisPaths,
            _: &Templater,
            _: &mut Task,
        ) -> anyhow::Result<()> {
            Ok(())
        }
    }

    // Helper function to setup generator registry with mocks
    fn setup_registry() -> GeneratorRegistry {
        let mut reg = GeneratorRegistry::default();
        reg.generators.push(Arc::new(MockGenerator {
            name: "alacritty",
            g_type: GeneratorType::Terminal,
            installed: true,
        }));
        reg.generators.push(Arc::new(MockGenerator {
            name: "zsh",
            g_type: GeneratorType::Prompt,
            installed: false,
        }));
        reg.generators.push(Arc::new(MockGenerator {
            name: "kitty",
            g_type: GeneratorType::Terminal,
            installed: true,
        }));

        reg.generators.sort_by(|a, b| {
            a.generator_type()
                .cmp(&b.generator_type())
                .then(a.name().cmp(b.name()))
        });
        reg
    }

    #[test]
    fn should_return_all_sorted_generators() {
        let reg = setup_registry();
        let all = reg.all();

        assert_eq!(all[0].name(), "alacritty");
        assert_eq!(all[1].name(), "kitty");
        assert_eq!(all[2].name(), "zsh");
    }

    #[test]
    fn should_handle_get_and_exists_for_registry() {
        let reg = setup_registry();

        assert!(reg.exists("alacritty"));
        assert!(!reg.exists("windows_terminal"));

        let gr = reg.get("kitty");
        assert!(gr.is_some());
        assert_eq!(gr.unwrap().name(), "kitty");
    }

    #[test]
    fn should_handle_filtering_by_type_for_generators() {
        let reg = setup_registry();
        let terminals = reg.by_type(GeneratorType::Terminal);

        assert_eq!(terminals.len(), 2);
        assert_eq!(terminals[0].name(), "alacritty");
        assert_eq!(terminals[1].name(), "kitty");
    }

    #[test]
    fn should_apply_installed_filters_for_generators() {
        let reg = setup_registry();
        let installed = reg.installed();

        assert_eq!(installed.len(), 2);
        assert!(installed.iter().any(|g| g.name() == "alacritty"));
        assert!(!installed.iter().any(|g| g.name() == "zsh"));
    }

    #[test]
    fn should_discover_unenabled_generators() {
        let reg = setup_registry();
        let mut state = State::default();

        state.enable_generator("alacritty");
        let unenabled = reg.discover_unenabled(&state);

        assert_eq!(unenabled.len(), 1);
        assert_eq!(unenabled[0].name(), "kitty");
    }

    #[test]
    fn should_test_generator_type_set() {
        let reg = setup_registry();
        let types = reg.types();

        assert_eq!(types.len(), 2);
        assert!(types.contains(&GeneratorType::Terminal));
        assert!(types.contains(&GeneratorType::Prompt));
    }

    #[test]
    fn should_return_array_string_of_active_generators() {
        let reg = setup_registry();
        let mut state = State::default();

        state.enable_generator("alacritty");
        state.enable_generator("zsh");
        state.enable_generator("kitty");

        let array = reg.active(&state);
        assert_eq!(array, "alacritty, zsh, kitty");
    }
}
