use crate::domain::AppError;

/// Port for writing to the system clipboard.
pub trait ClipboardWriter {
    /// Write text to the clipboard.
    fn write_text(&mut self, text: &str) -> Result<(), AppError>;
}

/// No-op clipboard implementation for commands that don't require clipboard access.
#[derive(Debug, Clone, Default)]
pub struct NoopClipboard;

impl ClipboardWriter for NoopClipboard {
    fn write_text(&mut self, _text: &str) -> Result<(), AppError> {
        Ok(())
    }
}
