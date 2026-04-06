use crate::{
    core::IrisContext,
    models::{Palette, State},
    modules::{Generator, GeneratorType, multiplexer, prompts, system, terminals, tools},
};
use colored::Colorize;
use std::collections::BTreeSet;

/// The list of all generators
#[derive(Default)]
pub struct GeneratorRegistry {
    pub generators: Vec<Box<dyn Generator>>,
}

impl GeneratorRegistry {
    /// Creates registy and appends all generators/modules
    pub fn new() -> Self {
        let mut generators: Vec<Box<dyn Generator>> = Vec::new();

        generators.extend(terminals::get_all());
        generators.extend(prompts::get_all());
        generators.extend(tools::get_all());
        generators.extend(system::get_all());
        generators.extend(multiplexer::get_all());

        Self { generators }
    }

    /// Access to all generators
    pub fn all(&self) -> &[Box<dyn Generator>] {
        &self.generators
    }

    /// Access to all generators sorted by type and then by name
    pub fn all_sorted(&self) -> Vec<&Box<dyn Generator>> {
        let mut gens: Vec<_> = self.generators.iter().collect();
        gens.sort_by(|a, b| {
            a.generator_type()
                .cmp(&b.generator_type())
                .then(a.name().cmp(b.name()))
        });

        gens
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
    pub fn by_type(&self, g_type: GeneratorType) -> Vec<&Box<dyn Generator>> {
        self.generators
            .iter()
            .filter(|g| g.generator_type() == g_type)
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
        self.generators
            .iter()
            .find(|g| g.name() == name)
            .map(|g| g.is_installed())
            .unwrap_or(false)
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
        if !ctx.log.quiet {
            println!("\n {} {}", "󰚗".magenta(), "Updating targets...".bold());
        }

        let enabled = &ctx.state.enabled_generators;
        let to_apply: Vec<_> = self
            .generators
            .iter()
            .filter(|g| enabled.contains(g.name()) && g.is_installed())
            .collect();

        let total = to_apply.len();
        let start_all = std::time::Instant::now();

        for (i, generator) in to_apply.iter().enumerate() {
            let is_last = i == total - 1;
            let mut task = ctx.log.step(generator.name(), 2);

            if let Err(e) = generator.apply(palette, ctx) {
                ctx.log
                    .error(&format!("Failed {}: {}", generator.name(), e), 1);
                return Err(e);
            }

            task.done(is_last);
        }

        println!(
            "\n {} All systems updated! {}\n",
            "󰄬".green().bold(),
            format!("[{:.2?}]", start_all.elapsed()).dimmed()
        );

        Ok(())
    }
}
