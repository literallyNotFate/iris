use std::io::Write;

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

/// Unit-tests for cursor guard
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_test_lifecycle_for_cursor_guard() {
        let mut buffer = Vec::new();
        {
            let _guard = CursorGuard::new(&mut buffer);
        }

        assert_eq!(buffer, b"\x1b[?25l\x1b[?25h");
    }
}
