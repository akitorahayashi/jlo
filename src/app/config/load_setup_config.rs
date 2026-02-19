//! Setup configuration loading from repository.

use crate::domain::AppError;
use crate::domain::setup::error::SetupError;
use crate::domain::setup::tools_config::{SetupConfig, parse_tools_config_content};
use crate::ports::RepositoryFilesystem;

/// Load and parse setup tools configuration from `.jlo/setup/tools.yml`.
pub fn load_setup_config(store: &impl RepositoryFilesystem) -> Result<SetupConfig, AppError> {
    let tools_yml = ".jlo/setup/tools.yml";
    if !store.file_exists(tools_yml) {
        return Err(SetupError::ConfigMissing.into());
    }

    let content = store.read_file(tools_yml)?;
    parse_tools_config_content(&content)
}
