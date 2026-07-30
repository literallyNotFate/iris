use include_dir::{Dir, include_dir};

pub static TEMPLATES_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/templates");
pub static RESOURCES_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/resources");
