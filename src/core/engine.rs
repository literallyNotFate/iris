use crate::{
    guards::FsRollbackGuard,
    infra::{IrisPaths, Templater},
    log::Activity,
    models::{HealthStatus, Theme},
    modules::Generator,
};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Content injection position (to the start/end of a file, after/before the string)
pub enum InjectionPosition {
    Start,
    End,
    Before(String),
    After(String),
}

/// The central execution engine responsible for rendering templates,
/// caching configurations, and orchestrating atomic theme applications.
pub struct IrisEngine<'a, 't> {
    pub paths: &'a IrisPaths,
    pub templater: &'a Templater,
    pub theme: &'t Theme,
}

impl<'a, 't> IrisEngine<'a, 't> {
    /// Initializes a new `IrisEngine` bound to shared paths and the template renderer
    pub fn new(paths: &'a IrisPaths, templater: &'a Templater, theme: &'t Theme) -> Self {
        Self {
            paths,
            templater,
            theme,
        }
    }

    /// Compiles a normalized template context for the given generator and theme.
    /// Ensures all hex values are correctly prefixed with exactly one `#`
    pub fn build_context<G: Generator + ?Sized>(&self, generator: &G) -> Result<tera::Context> {
        let mut context = tera::Context::new();
        if let serde_json::Value::Object(map) = serde_json::to_value(&self.theme.colors)? {
            for (key, val) in map {
                if let Some(s) = val.as_str() {
                    context.insert(key, &format!("#{}", s.trim_start_matches('#')));
                } else if key == "ansi" && val.is_array() {
                    context.insert("ansi", &val);
                }
            }
        }

        context.insert("theme_name", &self.theme.name.to_lowercase());
        generator.enrich_context(&mut context, self.theme)?;

        Ok(context)
    }

    /// Executes the entire deployment lifecycle of a theme for a given generator.
    /// Automatically manages rendering, caching, and atomic rollbacks via FsRollbackGuard
    pub fn execute_apply<G: Generator + ?Sized>(
        &self,
        generator: &G,
        log: &mut Activity,
    ) -> Result<()> {
        use colored::*;
        let theme = self.theme;
        generator.pre_apply(self)?;

        let context = self.build_context(generator)?;
        let cache_path: PathBuf = generator.cache_path(&self.paths, &theme.name.to_lowercase());
        let link_path: PathBuf = generator.link_path(&self.paths, &theme.name.to_lowercase());

        log.info(&format!(
            "Generating {} theme for {}",
            theme.name.yellow(),
            generator.name().bold().cyan()
        ));

        let rendered: String = self
            .templater
            .render(&generator.template_path(), &context)
            .with_context(|| format!("Failed to render `{}` theme template", generator.name()))?;

        self.atomic_commit(&cache_path, rendered.as_bytes())?;

        self.with_rollback(&link_path, || {
            generator
                .strategy()
                .apply(self, generator, &cache_path, &link_path, log)
        })
        .with_context(|| format!("Failed to apply theme for module `{}`", generator.name()))?;

        Ok(())
    }

    /// Automatically resolves and fixes detected environment or configuration issues
    /// for any generator implementing `Diagnosable` + `Generator`
    pub fn execute_fix<G: Generator + ?Sized>(
        &self,
        generator: &G,
        status: &HealthStatus,
        activity: &mut Activity,
    ) -> anyhow::Result<()> {
        match status {
            HealthStatus::Ok => Ok(()),
            HealthStatus::Issue(_severity, issue, _hint) => {
                let msg: String = format!("Repaired `{}` issue: {}", generator.name(), issue);
                activity.log.action(&msg, || {
                    self.execute_apply(generator, &mut activity.muted())
                })
            }
        }
    }

    /// Executes cleanup/clear for a specific generator
    pub fn execute_cleanup<G: Generator + ?Sized>(&self, generator: &G) -> Result<()> {
        generator.cleanup(self.paths)
    }

    /// Removes a specific theme for a generator from cache and configs
    pub fn execute_remove_theme<G: Generator + ?Sized>(&self, generator: &G) -> Result<()> {
        generator.remove_theme(self.paths, &self.theme.name)
    }

    /// Universal atomic write: creates a backup, writes to a temporary file, and renames the file
    fn atomic_commit(&self, dst: &Path, content: &[u8]) -> Result<()> {
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)
                .context("Failed to create all parent directories for copying")?;
        }

        let backup: PathBuf = dst.with_extension("bak");
        if dst.exists() {
            std::fs::copy(dst, &backup)
                .with_context(|| format!("Failed to copy file: {}", dst.to_string_lossy()))?;
        }

        let guard = FsRollbackGuard::new(dst.to_path_buf(), backup);
        let tmp: PathBuf = dst.with_extension(format!("tmp-{}", std::process::id()));

        std::fs::write(&tmp, content).context("Failed to write to a tmp file")?;
        std::fs::rename(&tmp, dst).context("Failed to rename tmp file")?;

        guard.commit();
        Ok(())
    }

    /// Atomic write to file
    pub fn atomic_write(&self, src: &Path, dst: &Path) -> Result<()> {
        self.atomic_commit(dst, &std::fs::read(src)?)
    }

    /// Creates or replaces a symlink atomically, preventing a "broken link" state
    pub fn atomic_symlink(&self, target: &Path, link: &Path) -> Result<()> {
        if let Some(parent) = link.parent() {
            std::fs::create_dir_all(parent)
                .context("Failed to create parent directory for symlink")?;
        }

        let tmp_link: PathBuf = link.with_extension(format!("tmp-sym-{}", std::process::id()));
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            symlink(target, &tmp_link).with_context(|| {
                format!("Failed to create temporary symlink: {}", tmp_link.display())
            })?;
        }

        std::fs::rename(&tmp_link, link).with_context(|| {
            let _ = std::fs::remove_file(&tmp_link);
            format!(
                "Failed to atomically replace symlink at: {}",
                link.display()
            )
        })?;

        Ok(())
    }

    /// Helper to inject setting into config
    pub fn inject_line(&self, path: &Path, line: &str, pos: InjectionPosition) -> Result<()> {
        let content: String = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(e) => {
                return Err(e).with_context(|| format!("Failed to read file: {}", path.display()));
            }
        };
        if content.contains(line) {
            return Ok(());
        }

        let new = match pos {
            InjectionPosition::Start => format!("{}\n{}", line, content),
            InjectionPosition::End => format!("{}\n{}", content.trim_end(), line),
            InjectionPosition::After(marker) => {
                if content.contains(&marker) {
                    content.replace(&marker, &format!("{}\n{}", marker, line))
                } else {
                    format!("{}\n{}\n{}", marker, line, content)
                }
            }
            InjectionPosition::Before(marker) => {
                if content.contains(&marker) {
                    content.replace(&marker, &format!("{}\n{}", line, marker))
                } else {
                    format!("{}\n{}\n{}", line, marker, content)
                }
            }
        };
        self.atomic_commit(path, new.trim().as_bytes())
    }

    /// Helper to inject entire block into config
    pub fn inject_block(&self, path: &Path, marker: &str, new_block: &str) -> Result<()> {
        let content: String = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(e) => {
                return Err(e).with_context(|| format!("Failed to read file: {}", path.display()));
            }
        };

        let new_content = crate::utils::replace_block(&content, marker, new_block);
        self.atomic_commit(path, new_content.trim().as_bytes())
    }

    /// Remove config key wrapper
    pub fn remove_key(&self, path: &Path, key: &str) -> Result<()> {
        let content: String = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(e) => {
                return Err(e).with_context(|| format!("Failed to read file: {}", path.display()));
            }
        };

        let new: String = crate::utils::remove_key(&content, key);
        self.atomic_commit(path, new.as_bytes())
    }

    /// Remove config marker wrapper
    pub fn remove_marker(&self, path: &Path, marker: &str) -> Result<()> {
        let content: String = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(e) => {
                return Err(e).with_context(|| format!("Failed to read file: {}", path.display()));
            }
        };

        let new: String = crate::utils::remove_marker(&content, marker);
        self.atomic_commit(path, new.as_bytes())
    }

    /// Wraps a file system operation in a rollback guard
    pub(crate) fn with_rollback<F>(&self, link_path: &PathBuf, op: F) -> Result<()>
    where
        F: FnOnce() -> Result<()>,
    {
        let backup_path: PathBuf = link_path.with_extension("bak");
        let guard = FsRollbackGuard::new(link_path.clone(), backup_path);

        op()?;

        guard.commit();
        Ok(())
    }
}
