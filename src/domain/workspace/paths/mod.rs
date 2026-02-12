//! Type-safe path catalog for workspace artifacts.
//!
//! All logical workspace paths are defined here. Business logic modules must use
//! these accessors instead of ad-hoc `.join("...")` chains.
//!
//! - [`jules`] — paths rooted under `.jules/` (runtime artifacts)
//! - [`jlo`] — paths rooted under `.jlo/` (control-plane)

pub mod jlo;
pub mod jules;

/// The `.jules/` workspace directory name.
pub const JULES_DIR: &str = ".jules";

/// The `.jlo/` control-plane directory name.
pub const JLO_DIR: &str = ".jlo";

/// The roles directory name.
pub const ROLES_DIR: &str = "roles";

/// The role definition file name.
pub const ROLE_FILENAME: &str = "role.yml";

/// The scheduled execution file name.
pub const SCHEDULED_FILENAME: &str = "scheduled.toml";

/// The version marker file name.
pub const VERSION_FILE: &str = ".jlo-version";
