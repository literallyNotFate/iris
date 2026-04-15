use anyhow::{Result, anyhow};
use include_dir::{Dir, include_dir};
use std::{ffi, fs, path::PathBuf};
use tera::{Context, Tera};

static TEMPLATES_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/templates");

/// Tera templater made for tool themes
#[derive(Clone)]
pub struct Templater {
    tera: Tera,
}

impl Templater {
    pub fn new(path: Option<PathBuf>) -> Self {
        let mut tera = Tera::default();

        let mut stack = vec![&TEMPLATES_DIR];
        let mut embedded = Vec::new();

        while let Some(dir) = stack.pop() {
            for entry in dir.entries() {
                match entry {
                    include_dir::DirEntry::Dir(d) => stack.push(d),
                    include_dir::DirEntry::File(f) => {
                        if f.path().extension() == Some(ffi::OsStr::new("hbs")) {
                            let id = f
                                .path()
                                .with_extension("")
                                .to_string_lossy()
                                .replace('\\', "/");

                            if let Some(content) = f.contents_utf8() {
                                embedded.push((id, content));
                            }
                        }
                    }
                }
            }
        }

        tera.add_raw_templates(embedded)
            .expect("Failed to load embedded templates");

        if let Some(p) = path.filter(|p| p.exists()) {
            Self::load_external_recursive(&mut tera, &p, &p);
        }

        Self { tera }
    }

    /// Renders the resulting templated based on context
    pub fn render(&self, template_name: &str, context: &Context) -> Result<String> {
        self.tera
            .render(template_name, context)
            .map_err(|e| anyhow!("Template error [{}]: {}", template_name, e))
    }

    /// Load all external templates recursively
    fn load_external_recursive(tera: &mut Tera, base: &PathBuf, current: &PathBuf) {
        let Ok(entries) = fs::read_dir(current) else {
            return;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                Self::load_external_recursive(tera, base, &path);
            } else if path.extension() == Some(ffi::OsStr::new("hbs")) {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(rel) = path.strip_prefix(base) {
                        let id = rel.with_extension("").to_string_lossy().replace('\\', "/");

                        let _ = tera.add_raw_template(&id, &content);
                    }
                }
            }
        }
    }
}

/// Unit-tests for templater
#[cfg(test)]
mod tests {
    use super::*;
    use tempdir::TempDir;
    use tera::Context;

    #[test]
    fn should_load_embedded_templates() {
        let templater = Templater::new(None);
        let mut context = Context::new();
        context.insert("theme_name", "test");
        let result = templater.render("tools/fzf", &context);

        match result {
            Ok(_) => assert!(true),
            Err(e) => {
                let err_msg = e.to_string();
                assert!(
                    !err_msg.contains("not found"),
                    "Template 'tools/fzf' should be found in embedded"
                );
            }
        }
    }

    #[test]
    fn should_override_with_custom_templates() {
        let temp_dir: TempDir = TempDir::new("templater_test").unwrap();
        let custom_template_path = temp_dir.path().join("custom_tool.hbs");

        fs::write(&custom_template_path, "Hello, {{ name }}!").unwrap();
        let templater = Templater::new(Some(temp_dir.path().to_path_buf()));

        let mut context = Context::new();
        context.insert("name", "Iris");

        let rendered = templater.render("custom_tool", &context).unwrap();
        assert_eq!(rendered, "Hello, Iris!");
    }

    #[test]
    fn should_handle_render_errors() {
        let temp_dir: TempDir = TempDir::new("templater_error").unwrap();
        let invalid_template_path = temp_dir.path().join("error.hbs");

        fs::write(&invalid_template_path, "Hello, {{ name").unwrap();

        let templater = Templater::new(Some(temp_dir.path().to_path_buf()));
        let context = Context::new();

        let result = templater.render("error", &context);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Template error [error]")
        );
    }

    #[test]
    fn should_load_nested_templates() {
        let temp_dir: TempDir = TempDir::new("templater_nested").unwrap();
        let nested_dir = temp_dir.path().join("subdir");
        fs::create_dir(&nested_dir).unwrap();

        let template_path = nested_dir.join("nested.hbs");
        fs::write(&template_path, "Nested: {{ val }}").unwrap();

        let templater = Templater::new(Some(temp_dir.path().to_path_buf()));
        let mut context = Context::new();
        context.insert("val", "ok");

        let rendered = templater
            .render("subdir/nested", &context)
            .expect("Should find nested template");
        assert_eq!(rendered, "Nested: ok");
    }

    #[test]
    fn should_ignore_non_hbs_files() {
        let temp_dir: TempDir = TempDir::new("templater_test").unwrap();
        let txt_path = temp_dir.path().join("ignore.txt");
        fs::write(&txt_path, "I should be ignored").unwrap();

        let templater = Templater::new(Some(temp_dir.path().to_path_buf()));
        let context = Context::new();

        let result = templater.render("ignore", &context);
        assert!(result.is_err());
    }
}
