//! Setup command module for jlo setup subcommands.

mod generate;
pub mod list;

pub use generate::execute as generate;
pub use list::{execute as list, execute_detail as list_detail};
