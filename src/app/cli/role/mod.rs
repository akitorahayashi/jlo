mod add;
mod create;
mod delete;
mod layer_selection;

use crate::domain::AppError;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum RoleCommands {
    /// Add a built-in role under .jlo/
    #[clap(visible_aliases = ["a", "ad"])]
    Add {
        /// Layer (observers, innovators)
        layer: Option<String>,
        /// Built-in role name(s)
        roles: Vec<String>,
    },
    /// Create a new role under .jlo/
    #[clap(visible_aliases = ["c", "cr"])]
    Create {
        /// Layer (observers, innovators)
        layer: Option<String>,
        /// Name for the new role
        role: Option<String>,
    },
    /// Delete a role from .jlo/
    #[clap(visible_aliases = ["d", "dl"])]
    Delete {
        /// Layer (observers, innovators)
        layer: Option<String>,
        /// Role name to delete
        role: Option<String>,
    },
}

pub fn run_role(command: RoleCommands) -> Result<(), AppError> {
    match command {
        RoleCommands::Add { layer, roles } => add::run(layer, roles),
        RoleCommands::Create { layer, role } => create::run(layer, role),
        RoleCommands::Delete { layer, role } => delete::run(layer, role),
    }
}
