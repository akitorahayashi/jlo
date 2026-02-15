mod git;
mod github;
mod jlo_store;
mod jules_client;
mod jules_store;
mod repository_filesystem;
mod role_template_store;
mod setup_component_catalog;
mod workspace_store;

pub use git::GitPort;
pub use github::{GitHubPort, IssueInfo, PrComment, PullRequestDetail, PullRequestInfo};
pub use jlo_store::JloStorePort;
pub use jules_client::{AutomationMode, JulesClient, SessionRequest, SessionResponse};
pub use jules_store::JulesStorePort;
pub use repository_filesystem::RepositoryFilesystemPort;
pub use role_template_store::{RoleTemplateStore, ScaffoldFile};
pub use setup_component_catalog::SetupComponentCatalog;
pub use workspace_store::{DiscoveredRole, WorkspaceStore};
