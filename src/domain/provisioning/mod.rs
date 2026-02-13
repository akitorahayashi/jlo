//! Script and config generator domain logic.

mod env_file;
mod install_script;

pub use env_file::SetupEnvArtifacts;

/// Domain logic for generating setup scripts and configuration files.
pub struct ArtifactFactory;

impl ArtifactFactory {
    /// Generate install.sh content from resolved components.
    pub fn generate_install_script(components: &[crate::domain::Component]) -> String {
        install_script::generate(components)
    }

    /// Generate or merge vars.toml and secrets.toml content.
    ///
    /// Preserves existing values while adding new keys from components.
    pub fn merge_env_artifacts(
        components: &[crate::domain::Component],
        existing_vars_toml: Option<&str>,
        existing_secrets_toml: Option<&str>,
    ) -> Result<SetupEnvArtifacts, crate::domain::AppError> {
        env_file::merge(components, existing_vars_toml, existing_secrets_toml)
    }
}
