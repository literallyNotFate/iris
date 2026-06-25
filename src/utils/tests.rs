use crate::{
    core::{IrisContext, IrisPaths, Templater},
    log::Activity,
    models::{HealthStatus, Theme},
    modules::{Generator, GeneratorType},
};
use tempdir::TempDir;

/// Mock entire context environment
pub fn mock_context() -> (TempDir, IrisContext) {
    use crate::{
        core::{IrisPaths, Templater},
        log::Logger,
        models::State,
        modules::GeneratorRegistry,
    };

    let temp_dir = TempDir::new("iris_test").expect("Failed to create temp directory");
    let root = temp_dir.path();

    let paths = IrisPaths {
        config: root.join(".config/iris"),
        cache: root.join(".cache/iris"),
        core: root.join(".cache/iris/core"),
        generators: root.join(".cache/iris/generators"),
        bin: root.join(".cache/iris/bin"),
        state_file: root.join(".config/iris/state.json"),
        current_theme: root.join(".cache/iris/core/current_theme"),
        themes: root.join(".cache/iris/core/themes"),
    };

    std::fs::create_dir_all(&paths.config).expect("Failed to create .config/iris");
    std::fs::create_dir_all(&paths.core).expect("Failed to create .cache/iris/core");
    std::fs::create_dir_all(&paths.themes).expect("Failed to create .cache/iris/core/themes");
    std::fs::create_dir_all(&paths.generators).expect("Failed to create .cache/iris/generators");
    std::fs::create_dir_all(&paths.bin).expect("Failed to create .cache/iris/bin");

    let templates_path = paths.config.join("templates");
    std::fs::create_dir_all(&templates_path).expect("Failed to create .config/iris/templates");

    let ctx = IrisContext {
        paths,
        state: State::default(),
        registry: GeneratorRegistry::default(),
        log: Logger::silent(),
        templater: Templater::new(Some(templates_path)),
    };

    (temp_dir, ctx)
}

/// Macro to skip the test case if app is not installed
#[macro_export]
macro_rules! skip_if_not_installed {
    ($executor:expr) => {
        if !$executor.is_installed() {
            println!(
                "cargo:warning=Skipping integration test for '{}': application not installed.",
                $executor.name()
            );

            return;
        }
    };
}

/// Generator mock for registry test
pub struct MockGenerator {
    pub name: &'static str,
    pub g_type: GeneratorType,
    pub installed: bool,
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
        _: &Theme,
        _: &IrisPaths,
        _: &Templater,
        _: &mut Activity,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn build_render_context(&self, _: &Theme) -> tera::Context {
        tera::Context::new()
    }

    fn fix(
        &self,
        _: &HealthStatus,
        _: &Theme,
        _: &IrisPaths,
        _: &Templater,
        _: &mut Activity,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}
