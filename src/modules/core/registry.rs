use crate::{
    core::IrisEngine,
    infra::{IrisPaths, Templater},
    log::{Logger, LoggingVerbosity},
    models::{HealthStatus, State, Theme},
    modules::{Generator, GeneratorType},
};
use anyhow::{Context, Result};
use colored::Colorize;
use rayon::prelude::*;
use std::{
    collections::{BTreeSet, HashMap},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

/// Manage all generators
#[derive(Default, Clone)]
pub struct GeneratorRegistry {
    pub generators: Vec<Arc<dyn Generator>>,
    index: HashMap<String, usize>,
}

impl GeneratorRegistry {
    /// Creates registry and appends all generators/modules
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

        let index: HashMap<String, usize> = generators
            .iter()
            .enumerate()
            .map(|(i, g)| (g.name().to_string(), i))
            .collect();

        Self { generators, index }
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

    /// List of all unique types that are enabled in registry
    pub fn types(&self) -> BTreeSet<GeneratorType> {
        self.generators.iter().map(|g| g.generator_type()).collect()
    }

    /// Get generator by name
    pub fn get(&self, name: &str) -> Option<&dyn Generator> {
        self.index.get(name).map(|&i| self.generators[i].as_ref())
    }

    /// Get generator by name or return an error if it doesn't exist
    pub fn get_required(&self, name: &str) -> Result<&dyn Generator> {
        self.get(name)
            .ok_or_else(|| anyhow::anyhow!("Generator `{}` not found in registry!", name))
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

    /// Get all generators that are both enabled by user and installed on the system
    pub fn enabled_and_installed(&self, state: &State) -> Vec<&dyn Generator> {
        self.generators
            .iter()
            .filter(|g| state.is_enabled(g.name()) && g.is_installed())
            .map(|b| b.as_ref())
            .collect()
    }

    /// Check whether this generator is installed in system
    pub fn is_installed(&self, name: &str) -> bool {
        self.get(name).map(|g| g.is_installed()).unwrap_or(false)
    }

    /// Checks whether this generator exists
    pub fn exists(&self, name: &str) -> bool {
        self.index.contains_key(name)
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

    /// Runs health checks on all enabled generators and returns tuple of (healthy, errors)
    pub fn check_all<'a>(
        &'a self,
        state: &State,
        paths: &IrisPaths,
        theme: &str,
    ) -> (
        Vec<(&'a dyn Generator, HealthStatus)>,
        Vec<(&'a dyn Generator, HealthStatus)>,
    ) {
        let mut healthy = Vec::new();
        let mut errors = Vec::new();

        for generator in self.generators.iter().map(|b| b.as_ref()) {
            if !state.is_enabled(generator.name()) {
                continue;
            }

            let status = generator.health_check(paths, theme);
            if status.is_ok() {
                healthy.push((generator, status));
            } else {
                errors.push((generator, status));
            }
        }

        (healthy, errors)
    }
}

impl GeneratorRegistry {
    /// Apply themes to available programs (enabled generators)
    pub fn apply_all(
        &self,
        theme: &Theme,
        state: &State,
        paths: &IrisPaths,
        templater: &Templater,
        log: &Logger,
    ) -> Result<()> {
        let to_apply: Vec<&dyn Generator> = self
            .generators
            .iter()
            .filter(|g| state.is_enabled(g.name()) && g.is_installed())
            .map(|b| b.as_ref())
            .collect();

        if to_apply.is_empty() {
            log.warn("No targets enabled or installed!");
            return Ok(());
        }

        let engine: IrisEngine = IrisEngine::new(paths, templater, theme);
        let total: usize = to_apply.len();

        if log.verbosity == LoggingVerbosity::Silent {
            for generator in &to_apply {
                let mut silent_activity = log.activity();

                engine.execute_apply(*generator, &mut silent_activity)?;
            }
            return Ok(());
        }

        let root = log.step_with_icon(
            "󰛓".blue().bold(),
            &format!("Updating {} targets ...", total.to_string().blue().bold()),
            true,
        );

        for (i, generator) in to_apply.iter().enumerate() {
            let is_last = i == total - 1;

            if log.verbosity == LoggingVerbosity::Minimal {
                let mut silent_sub_activity = root.muted();
                engine.execute_apply(*generator, &mut silent_sub_activity)?;
            } else {
                let generator_color = generator.generator_type().color();
                let generator_icon = generator
                    .generator_type()
                    .icon()
                    .color(generator_color)
                    .bold();

                let mut activity = root.log.step_with_icon(
                    generator_icon,
                    &format!("{}", generator.name().color(generator_color).bold()),
                    is_last,
                );

                engine
                    .execute_apply(*generator, &mut activity)
                    .with_context(|| {
                        format!(
                            "Failed to apply theme to `{}`",
                            generator.name().bold().green()
                        )
                    })?;

                activity.done_with(&format!("{} updated!", generator.name().cyan()));
            }
        }

        root.done_with(&format!(
            "All {} targets were updated!",
            total.to_string().blue()
        ));

        Ok(())
    }

    /// Apply themes to available programs in parallel (clean log version)
    pub fn apply_all_parallel(
        &self,
        theme: &Theme,
        state: &State,
        paths: &IrisPaths,
        templater: &Templater,
        log: &Logger,
    ) -> Result<()> {
        let to_apply: Vec<&dyn Generator> = self
            .generators
            .iter()
            .filter(|g| state.is_enabled(g.name()) && g.is_installed())
            .map(|b| b.as_ref())
            .collect();

        if to_apply.is_empty() {
            log.warn("No targets enabled or installed!");
            return Ok(());
        }

        let total: usize = to_apply.len();
        if log.verbosity == LoggingVerbosity::Silent {
            to_apply.par_iter().try_for_each(|generator| {
                let engine = IrisEngine::new(paths, templater, theme);
                let mut silent_activity = log.activity();
                engine.execute_apply(*generator, &mut silent_activity)
            })?;
            return Ok(());
        }

        let completed = AtomicUsize::new(0);
        println!(
            "{} Updating {} targets in parallel ...\n",
            "󰛓".blue().bold(),
            total.to_string().blue().bold()
        );

        let engine = IrisEngine::new(paths, templater, theme);
        let results: Result<Vec<()>, anyhow::Error> = to_apply
            .par_iter()
            .map(|generator| {
                let mut silent_activity = crate::log::Activity::silent();
                let res = engine
                    .execute_apply(*generator, &mut silent_activity)
                    .with_context(|| {
                        format!(
                            "Failed to apply theme to `{}`",
                            generator.name().bold().green()
                        )
                    });

                if res.is_ok() {
                    let current = completed.fetch_add(1, Ordering::SeqCst) + 1;
                    println!(
                        "  {} [{}/{}] Target `{}` updated!",
                        "✓".green().bold(),
                        current.to_string().yellow(),
                        total,
                        generator.name().cyan()
                    );
                }

                res
            })
            .collect();

        results?;

        println!(
            "\n{}",
            "✓ All targets were updated successfully in parallel!"
                .green()
                .bold(),
        );
        Ok(())
    }
}

/// Unit-tests for generator registry
#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::{Strategy, generator::GeneratorMock};

    /// Helper function to setup generator registry with mocks
    fn setup_registry() -> GeneratorRegistry {
        let mut reg = GeneratorRegistry::default();
        reg.generators.push(Arc::new(GeneratorMock {
            name: "alacritty",
            g_type: GeneratorType::Terminal,
            installed: true,
            strategy: Strategy::Symlink,
        }));
        reg.generators.push(Arc::new(GeneratorMock {
            name: "zsh",
            g_type: GeneratorType::Prompt,
            installed: false,
            strategy: Strategy::Pipeline { steps: vec![] },
        }));
        reg.generators.push(Arc::new(GeneratorMock {
            name: "kitty",
            g_type: GeneratorType::Terminal,
            installed: true,
            strategy: Strategy::Symlink,
        }));

        reg.generators.sort_by(|a, b| {
            a.generator_type()
                .cmp(&b.generator_type())
                .then(a.name().cmp(b.name()))
        });

        reg.index = reg
            .generators
            .iter()
            .enumerate()
            .map(|(i, g)| (g.name().to_string(), i))
            .collect();

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

        let array: String = reg.active(&state);
        let expected: String = format!(
            "{}, {}, {}",
            "alacritty".cyan(),
            "kitty".cyan(),
            "zsh".cyan()
        );

        assert_eq!(array, expected);
    }
}
