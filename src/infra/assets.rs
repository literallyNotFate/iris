use include_dir::{Dir, include_dir};

pub static TEMPLATES_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/templates");
pub static RESOURCES_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/resources");

/// Static list of `nvim` builtin themes to avoid heavy runtime Lua/process execution
pub const BUILTIN_NVIM_THEMES: &[&str] = &[
    "blue",
    "darkblue",
    "default",
    "delek",
    "desert",
    "elflord",
    "evening",
    "habamax",
    "industry",
    "koehler",
    "lunaperche",
    "morning",
    "murphy",
    "pablo",
    "peachpuff",
    "quiet",
    "retrobox",
    "ron",
    "shine",
    "slate",
    "sorbet",
    "torte",
    "unokai",
    "wildcharm",
    "zaibatsu",
    "zellner",
];
