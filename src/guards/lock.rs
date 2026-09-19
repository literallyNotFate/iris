use crate::infra::IrisPaths;
use anyhow::{Context, Result};
use std::{
    fs::{self, File, OpenOptions},
    path::PathBuf,
};

/// Guard to prevent multiple simultaneous instances of application from running
pub struct LockGuard {
    file: File,
    lock_path: PathBuf,
}

impl LockGuard {
    pub fn acquire(paths: &IrisPaths) -> Result<Self> {
        let lock_path: PathBuf = paths.cache.join("iris.lock");
        if let Some(parent) = lock_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&lock_path)
            .context("Failed to create application lock file")?;

        file.try_lock()
            .context("Another instance of `iris` is already running!")?;

        Ok(Self { file, lock_path })
    }
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        let _ = self.file.unlock();
        let _ = fs::remove_file(&self.lock_path);
    }
}

/// Unit-tests for lock guard
#[cfg(test)]
mod tests {
    use super::*;
    use tempdir::TempDir;

    #[test]
    fn should_prevent_concurrent_acquisition() {
        let temp_dir = TempDir::new("lock_test").unwrap();
        let base = temp_dir.path();

        let paths =
            IrisPaths::with_base(base.join("config"), base.join("cache"), base.join("home"));

        fs::create_dir_all(&paths.cache).unwrap();
        let _guard1 = LockGuard::acquire(&paths).unwrap();
        let guard2_result = LockGuard::acquire(&paths);
        assert!(
            guard2_result.is_err(),
            "Expected error when acquiring a locked resource, but succeeded"
        );
    }

    #[test]
    fn should_allow_acquisition_after_drop() {
        let temp_dir = TempDir::new("lock_test").unwrap();
        let base = temp_dir.path();

        let paths =
            IrisPaths::with_base(base.join("config"), base.join("cache"), base.join("home"));

        fs::create_dir_all(&paths.cache).unwrap();
        {
            let _guard1 = LockGuard::acquire(&paths).unwrap();
        }

        let guard2_result = LockGuard::acquire(&paths);
        assert!(
            guard2_result.is_ok(),
            "Expected successful acquisition after previous guard was dropped"
        );
    }
}
