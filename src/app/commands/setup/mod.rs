//! Setup command module for jlo setup subcommands.

mod generate;
mod init;
pub mod list;

pub use generate::execute as generate;
pub use init::execute as init;
pub use list::{execute as list, execute_detail as list_detail};
