pub mod add_item;
pub mod delete_item;
pub mod list_items;

use crate::error::AppError;
use crate::storage::Storage;

#[cfg(test)]
pub(crate) mod test_support;

pub(crate) trait Execute<R> {
    fn execute(&self, storage: &impl Storage) -> Result<R, AppError>;
}
