use crate::{
    core::{IrisContext, IrisPaths, Templater},
    models::{Palette, State},
    modules::GeneratorRegistry,
    ui::Logger,
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
        state_file: root.join(".config/iris/state.json"),
        current_theme: root.join(".cache/iris/current_theme"),
    };

    fs::create_dir_all(&paths.config).unwrap();
    fs::create_dir_all(&paths.cache).unwrap();

    let user_templates_path = paths.config.join("templates");
    fs::create_dir_all(&user_templates_path).unwrap();

    let ctx = IrisContext {
        paths,
        state: State::default(),
        registry: GeneratorRegistry::default(),
        log: Logger::new(true),
        templater: Templater::new(Some(user_templates_path)),
    };

    (temp_dir, ctx)
}

impl Palette {
    /// Function to create palette mock
    pub fn mock() -> Self {
        Self {
            name: "test-theme".into(),
            bg: "#1a1b26".into(),
            fg: "#c0caf5".into(),
            caret: "#c0caf5".into(),
            line_hl: "#292e42".into(),
            sel: "#334455".into(),
            gutter_fg: "#3b4261".into(),
            comment: "#565f89".into(),
            variable: "#bb9af7".into(),
            constant: "#ff9e64".into(),
            number: "#ff9e64".into(),
            string: "#9ece6a".into(),
            keyword: "#7aa2f7".into(),
            operator: "#89ddff".into(),
            func: "#7ad6ff".into(),
            type_name: "#2ac3de".into(),
            tag: "#f7768e".into(),
            attribute: "#e0af68".into(),
            white: "#ffffff".into(),
            ansi: vec![
                "#15161e".into(),
                "#f7768e".into(),
                "#9ece6a".into(),
                "#e0af68".into(),
                "#7aa2f7".into(),
                "#bb9af7".into(),
                "#7dcfff".into(),
                "#a9b1d6".into(),
                "#414868".into(),
                "#f7768e".into(),
                "#9ece6a".into(),
                "#e0af68".into(),
                "#7aa2f7".into(),
                "#bb9af7".into(),
                "#7dcfff".into(),
                "#c0caf5".into(),
            ],
        }
    }
}
