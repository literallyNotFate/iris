use std::{fs, io::Write, path::PathBuf};

/// Transaction guard for safe filesystem actions.
/// If there is a problem/panic in scope - rollback to initial state
pub struct FsRollbackGuard {
    target_path: PathBuf,
    backup_path: PathBuf,
    existed_before: bool,
    success: bool,
}

impl FsRollbackGuard {
    pub fn new(target_path: PathBuf, backup_path: PathBuf) -> Self {
        let existed_before = target_path.exists();
        Self {
            target_path,
            backup_path,
            existed_before,
            success: false,
        }
    }

    /// Marks operation as completed.
    /// Backup deletion and automatic rollback with Drop cancels
    pub fn commit(mut self) {
        self.success = true;
        if self.backup_path.exists() {
            let _ = fs::remove_file(&self.backup_path);
        }
    }
}

impl Drop for FsRollbackGuard {
    fn drop(&mut self) {
        if !self.success {
            if self.existed_before {
                if self.backup_path.exists() {
                    let _ = fs::copy(&self.backup_path, &self.target_path);
                    let _ = fs::remove_file(&self.backup_path);
                }
            } else {
                if self.target_path.exists() {
                    let _ = fs::remove_file(&self.target_path);
                }
            }
        }
    }
}

/// Guard for cursor visibility management.
/// Works with everyting that implements `Write` (terminal, buffer, vector)
pub struct CursorGuard<W: Write> {
    writer: W,
}

impl<W: Write> CursorGuard<W> {
    pub fn new(mut writer: W) -> Self {
        let _ = write!(writer, "\x1b[?25l");
        let _ = writer.flush();
        Self { writer }
    }
}

impl<W: Write> Drop for CursorGuard<W> {
    fn drop(&mut self) {
        let _ = write!(self.writer, "\x1b[?25h");
        let _ = self.writer.flush();
    }
}

/// Unit-tests for all guards
#[cfg(test)]
mod tests {
    use super::*;
    use tempdir::TempDir;

    #[test]
    fn should_revert_changes_on_failure_for_fs_rollback_guard() {
        let temp_dir: TempDir = TempDir::new("rollback_test").unwrap();
        let target = temp_dir.path().join("config.lua");
        let backup = temp_dir.path().join("config.lua.bak");

        fs::write(&target, "old_content").unwrap();
        fs::copy(&target, &backup).unwrap();

        {
            let _guard = FsRollbackGuard::new(target.clone(), backup.clone());
            fs::write(&target, "corrupted_content").unwrap();
        }

        assert_eq!(fs::read_to_string(&target).unwrap(), "old_content");
        assert!(!backup.exists());
    }

    #[test]
    fn should_handle_delete_if_not_existed_for_fs_rollback_guard() {
        let temp_dir: TempDir = TempDir::new("rollback_test").unwrap();
        let target = temp_dir.path().join("new_config.lua");
        let backup = temp_dir.path().join("new_config.lua.bak");

        {
            let _guard = FsRollbackGuard::new(target.clone(), backup.clone());
            fs::write(&target, "partial_content").unwrap();
        }

        assert!(!target.exists());
    }

    #[test]
    fn should_test_lifecycle_for_cursor_guard() {
        let mut buffer = Vec::new();
        {
            let _guard = CursorGuard::new(&mut buffer);
        }

        assert_eq!(buffer, b"\x1b[?25l\x1b[?25h");
    }
}
