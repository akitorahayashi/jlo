pub mod configuration;
pub mod error;
pub mod identifiers;
pub mod issue;

pub mod prompt;
pub mod workspace;

pub mod builtin_role;
pub mod component_graph;
pub mod setup_artifacts;

pub use builtin_role::BuiltinRoleEntry;
pub use configuration::{
    ExecutionConfig, JulesApiConfig, MockConfig, MockOutput, RunConfig, Schedule,
    WorkflowRunnerMode,
};
pub use error::{AppError, IoErrorKind};
pub use identifiers::{ComponentId, RoleId};
pub use issue::IssueHeader;

pub use prompt::{
    PromptAssemblyError, PromptAssetLoader, PromptContext, assemble_prompt, assemble_with_issue,
};
pub use workspace::{
    Component, EnvSpec, JLO_DIR, JULES_DIR, Layer, ScaffoldManifest, VERSION_FILE,
};
