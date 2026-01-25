use crate::commands::Execute;
use crate::error::AppError;
use crate::storage::Storage;

/// Example command demonstrating how to read data from the storage layer.
pub struct ListItems;

impl Execute<Vec<String>> for ListItems {
    fn execute(&self, storage: &impl Storage) -> Result<Vec<String>, AppError> {
        storage.list_items()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::test_support::MockStorage;

    #[test]
    fn list_items_returns_storage_values() {
        let storage = MockStorage::default();
        storage.set_list_items(["first", "second"]);

        let items = ListItems.execute(&storage).expect("execution should succeed");
        assert_eq!(items, vec!["first".to_string(), "second".to_string()]);
    }
}
