use arboard::Clipboard;

use crate::domain::AppError;
use crate::ports::ClipboardWriter;

/// Arboard-based clipboard implementation.
pub struct ArboardClipboardWriter {
    clipboard: Clipboard,
}

impl ArboardClipboardWriter {
    /// Create a new arboard clipboard instance.
    pub fn new() -> Result<Self, AppError> {
        let clipboard = Clipboard::new().map_err(|e| AppError::ClipboardError(format!("{}", e)))?;
        Ok(Self { clipboard })
    }
}

impl ClipboardWriter for ArboardClipboardWriter {
    fn write_text(&mut self, text: &str) -> Result<(), AppError> {
        self.clipboard.set_text(text).map_err(|e| AppError::ClipboardError(format!("{}", e)))
    }
}
