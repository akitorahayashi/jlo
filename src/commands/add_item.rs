use crate::commands::Execute;
use crate::error::AppError;
use crate::storage::Storage;

/// Minimal example command illustrating how to write to the storage layer.
pub struct AddItem<'a> {
    pub id: &'a str,
    pub content: &'a str,
}

impl Execute<()> for AddItem<'_> {
    fn execute(&self, storage: &impl Storage) -> Result<(), AppError> {
        storage.add_item(self.id, self.content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::test_support::MockStorage;

    #[test]
    fn add_item_forwards_to_storage() {
        let storage = MockStorage::default();
        let command = AddItem { id: "demo", content: "example" };

        command.execute(&storage).expect("execution should succeed");

        let calls = storage.add_calls.borrow();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0], ("demo".to_string(), "example".to_string()));
    }
}
