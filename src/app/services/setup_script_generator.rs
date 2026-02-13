//! Setup script and config generator service.

use crate::domain::setup_artifacts::ArtifactFactory;
use crate::domain::setup_artifacts::SetupEnvArtifacts;
use crate::domain::{AppError, Component};

/// Service for generating setup scripts and configuration files.
pub struct SetupScriptGenerator;

impl SetupScriptGenerator {
    /// Generate install.sh content from resolved components.
    ///
    /// Delegates to domain logic.
    pub fn generate_install_script(components: &[Component]) -> String {
        ArtifactFactory::generate_install_script(components)
    }

    /// Generate or merge vars.toml and secrets.toml content.
    ///
    /// Preserves existing values while adding new keys from components.
    /// Delegates to domain logic.
    pub fn merge_env_artifacts(
        components: &[Component],
        existing_vars_toml: Option<&str>,
        existing_secrets_toml: Option<&str>,
    ) -> Result<SetupEnvArtifacts, AppError> {
        ArtifactFactory::merge_env_artifacts(components, existing_vars_toml, existing_secrets_toml)
    }
}
