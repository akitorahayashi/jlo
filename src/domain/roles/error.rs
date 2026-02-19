/// Role capability error.
#[derive(Debug, thiserror::Error)]
pub enum RoleError {
    #[error("Invalid role identifier '{0}': must be alphanumeric with hyphens or underscores")]
    InvalidId(String),

    #[error(
        "Invalid layer '{name}': must be one of Narrator, Observers, Decider, Planner, Implementer, Innovators, Integrator"
    )]
    InvalidLayer { name: String },

    #[error("Role '{0}' not found")]
    NotFound(String),

    #[error("Role '{role}' already exists in layer '{layer}'")]
    AlreadyExists { role: String, layer: String },

    #[error("Duplicate role '{0}' specified")]
    DuplicateRequest(String),

    #[error("Role '{role}' not found in config for layer '{layer}'")]
    NotInConfig { role: String, layer: String },

    #[error("Layer '{0}' is single-role and does not support custom roles. Use the built-in role.")]
    SingleRoleLayerTemplate(String),
}
