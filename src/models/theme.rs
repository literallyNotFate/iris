use crate::context::AppContext;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::{collections::BTreeMap, fs};

/// Theme
#[derive(Deserialize, Debug)]
pub struct Theme {
    pub name: String,
    pub colors: BTreeMap<String, String>,
    pub palette: BTreeMap<String, String>,
}

impl Theme {
    /// Load theme from file by name
    pub fn load_by_name(name: &str, ctx: &AppContext) -> Result<Self> {
        let path = ctx.theme_path(name);

        let content = fs::read_to_string(&path)
            .with_context(|| format!("Theme file for '{}' not found at {:?}", name, path))?;

        let theme: Self = toml::from_str(&content)
            .with_context(|| format!("Failed to parse TOML for theme '{}'", name))?;

        Ok(theme)
    }

    /// Fast access to color from palette
    pub fn color(&self, key: &str) -> String {
        self.colors
            .get(key)
            .or_else(|| self.palette.get(key))
            .cloned()
            .unwrap_or_else(|| "#ffffff".to_string())
    }
}
