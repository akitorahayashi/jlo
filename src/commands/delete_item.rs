use crate::commands::Execute;
use crate::error::AppError;
use crate::storage::Storage;

/// Example command for removing an item from storage.
pub struct DeleteItem<'a> {
    pub id: &'a str,
}

impl Execute<()> for DeleteItem<'_> {
    fn execute(&self, storage: &impl Storage) -> Result<(), AppError> {
        storage.delete_item(self.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::test_support::MockStorage;

    #[test]
    fn delete_item_forwards_to_storage() {
        let storage = MockStorage::default();
        let command = DeleteItem { id: "demo" };

        command.execute(&storage).expect("execution should succeed");

        let calls = storage.delete_calls.borrow();
        assert_eq!(calls.as_slice(), ["demo".to_string()]);
    }
}
