pub mod configuration;
pub mod error;
pub mod identifiers;
pub mod requirement;

pub mod prompt_assembly;
pub mod workspace;

pub mod builtin_role;
pub mod component_graph;
pub mod setup_artifacts;

pub use builtin_role::BuiltinRoleEntry;
pub use configuration::{
    ExecutionConfig, JulesApiConfig, MockConfig, MockOutput, RunConfig, Schedule,
    WorkflowTimingConfig, WorkflowRunnerMode,
};
pub use error::{AppError, IoErrorKind};
pub use identifiers::{ComponentId, RoleId};
pub use requirement::RequirementHeader;

pub use prompt_assembly::{
    PromptAssemblyError, PromptAssetLoader, PromptContext, assemble_prompt, assemble_with_issue,
};
pub use workspace::{
    Component, EnvSpec, JLO_DIR, JULES_DIR, Layer, ScaffoldManifest, VERSION_FILE,
};
