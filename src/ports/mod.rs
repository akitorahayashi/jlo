mod component_catalog;
mod git;
mod github;
mod jules_client;
mod role_template_store;
mod workspace_store;

pub use component_catalog::ComponentCatalog;
pub use git::GitPort;
pub use github::{GitHubPort, IssueInfo, PrComment, PullRequestDetail, PullRequestInfo};
pub use jules_client::{AutomationMode, JulesClient, SessionRequest, SessionResponse};
pub use role_template_store::{RoleTemplateStore, ScaffoldFile};
pub use workspace_store::{DiscoveredRole, WorkspaceStore};
