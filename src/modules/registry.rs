use crate::{
    core::IrisContext,
    models::{Palette, State},
    modules::{Generator, GeneratorType, multiplexer, prompts, system, terminals, tools},
};
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
}

impl GeneratorRegistry {
    /// Apply themes to available programs (enabled generators)
    pub fn apply_all(&self, palette: &Palette, ctx: &IrisContext) -> anyhow::Result<()> {
        let to_apply: Vec<&dyn Generator> = self
            .generators
            .iter()
            .filter(|g| ctx.state.is_enabled(g.name()) && g.is_installed())
            .map(|b| b.as_ref())
            .collect();

        if to_apply.is_empty() {
            return Ok(());
        }

        if !ctx.log.quiet {
            println!("\n {} {}", "󰚗".magenta(), "Updating targets...".bold());
        }

        let total = to_apply.len();
        let start_all = std::time::Instant::now();

        for (i, generator) in to_apply.iter().enumerate() {
            let mut task = ctx.log.step(generator.name(), 2);

            generator.apply(palette, ctx).map_err(|e| {
                ctx.log
                    .error(&format!("Failed {}: {}", generator.name(), e), 1);
                e
            })?;

            task.done(i == total - 1);
        }

        println!(
            "\n {} All systems updated! {}\n",
            "󰄬".green().bold(),
            format!("[{:.2?}]", start_all.elapsed()).dimmed()
        );

        Ok(())
    }
}

/// Unit-tests for generator registry
#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::HealthStatus;

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

        fn apply(&self, _: &Palette, _: &IrisContext) -> anyhow::Result<()> {
            Ok(())
        }

        fn build_render_context(&self, _: &Palette) -> tera::Context {
            tera::Context::new()
        }

        fn fix(&self, _: &HealthStatus, _: &Palette, _: &IrisContext) -> anyhow::Result<()> {
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
}
