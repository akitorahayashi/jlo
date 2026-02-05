mod component_catalog;
mod git;
mod github;
mod jules;
mod role_template;
mod workspace;

pub use component_catalog::ComponentCatalogPort;
pub use git::{CommitInfo, DiffStat, GitPort};
pub use github::{GitHubPort, PullRequestInfo};
pub use jules::{AutomationMode, JulesPort, SessionRequest, SessionResponse};
pub use role_template::{RoleTemplatePort, ScaffoldFile};
pub use workspace::{DiscoveredRole, WorkspacePort};
