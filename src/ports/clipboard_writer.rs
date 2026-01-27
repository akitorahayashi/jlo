use crate::domain::AppError;

/// Port for writing to the system clipboard.
pub trait ClipboardWriter {
    /// Write text to the clipboard.
    fn write_text(&mut self, text: &str) -> Result<(), AppError>;
}
