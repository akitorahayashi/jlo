use std::cell::RefCell;

use crate::domain::AppError;
use crate::ports::ClipboardWriter;

/// Mock clipboard for testing.
#[derive(Default)]
#[allow(dead_code)]
pub struct MockClipboard {
    pub written_text: RefCell<Option<String>>,
    pub should_fail: RefCell<bool>,
}

#[allow(dead_code)]
impl MockClipboard {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_should_fail(&self, fail: bool) {
        *self.should_fail.borrow_mut() = fail;
    }

    pub fn get_written_text(&self) -> Option<String> {
        self.written_text.borrow().clone()
    }
}

impl ClipboardWriter for MockClipboard {
    fn write_text(&mut self, text: &str) -> Result<(), AppError> {
        if *self.should_fail.borrow() {
            return Err(AppError::ClipboardError("Mock clipboard error".to_string()));
        }
        *self.written_text.borrow_mut() = Some(text.to_string());
        Ok(())
    }
}
