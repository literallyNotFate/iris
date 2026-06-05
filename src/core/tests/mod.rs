use crate::{
    core::{IrisContext, IrisPaths, Templater},
    log::Logger,
    models::State,
    modules::GeneratorRegistry,
};
use std::fs;
use tempdir::TempDir;

/// Setup test context environment
pub fn create_test_context() -> (TempDir, IrisContext) {
    let temp_dir = TempDir::new("iris_test").expect("Failed to create temp dir");
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

    fs::create_dir_all(&paths.config).unwrap();
    fs::create_dir_all(&paths.core).unwrap();
    fs::create_dir_all(&paths.themes).unwrap();
    fs::create_dir_all(&paths.generators).unwrap();
    fs::create_dir_all(&paths.bin).unwrap();

    let user_templates_path = paths.config.join("templates");
    fs::create_dir_all(&user_templates_path).unwrap();

    let ctx = IrisContext {
        paths,
        state: State::default(),
        registry: GeneratorRegistry::default(),
        log: Logger::silent(),
        templater: Templater::new(Some(user_templates_path)),
    };

    (temp_dir, ctx)
}
