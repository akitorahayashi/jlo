use crate::error::AppError;
use crate::storage::Storage;
use std::cell::RefCell;

#[derive(Default)]
pub(crate) struct MockStorage {
    pub add_calls: RefCell<Vec<(String, String)>>,
    pub delete_calls: RefCell<Vec<String>>,
    pub list_items_values: RefCell<Vec<String>>,
}

impl MockStorage {
    pub fn set_list_items<I>(&self, items: I)
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        let mut values = self.list_items_values.borrow_mut();
        values.clear();
        values.extend(items.into_iter().map(Into::into));
    }
}

impl Storage for MockStorage {
    fn add_item(&self, id: &str, content: &str) -> Result<(), AppError> {
        self.add_calls.borrow_mut().push((id.to_string(), content.to_string()));
        Ok(())
    }

    fn list_items(&self) -> Result<Vec<String>, AppError> {
        Ok(self.list_items_values.borrow().clone())
    }

    fn delete_item(&self, id: &str) -> Result<(), AppError> {
        self.delete_calls.borrow_mut().push(id.to_string());
        Ok(())
    }
}
