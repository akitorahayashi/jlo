mod git;
mod github;
mod jules_client;
mod role_template_store;
mod setup_component_catalog;
mod workspace_store;

pub use git::GitPort;
pub use github::{GitHubPort, IssueInfo, PrComment, PullRequestDetail, PullRequestInfo};
pub use jules_client::{AutomationMode, JulesClient, SessionRequest, SessionResponse};
pub use role_template_store::{RoleTemplateStore, ScaffoldFile};
pub use setup_component_catalog::SetupComponentCatalog;
pub use workspace_store::{DiscoveredRole, WorkspaceStore};
