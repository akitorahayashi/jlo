//! Script and config generator service.

use crate::domain::artifacts::ArtifactFactory;
use crate::domain::{AppError, Component};

/// Service for generating setup scripts and configuration files.
pub struct ArtifactGenerator;

impl ArtifactGenerator {
    /// Generate install.sh content from resolved components.
    ///
    /// Delegates to domain logic.
    pub fn generate_install_script(components: &[Component]) -> String {
        ArtifactFactory::generate_install_script(components)
    }

    /// Generate or merge env.toml content.
    ///
    /// Preserves existing values while adding new keys from components.
    /// Delegates to domain logic.
    pub fn merge_env_toml(
        components: &[Component],
        existing_content: Option<&str>,
    ) -> Result<String, AppError> {
        ArtifactFactory::merge_env_toml(components, existing_content)
    }
}
