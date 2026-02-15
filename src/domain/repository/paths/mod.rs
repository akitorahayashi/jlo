//! Type-safe path catalog for repository artifacts.
//!
//! All logical repository paths are defined here. Business logic modules must use
//! these accessors instead of ad-hoc `.join("...")` chains.
//!
//! - [`jules`] — paths rooted under `.jules/` (runtime artifacts)
//! - [`jlo`] — paths rooted under `.jlo/` (control-plane)

pub mod jlo;
pub mod jules;

/// The `.jules/` runtime directory name.
pub const JULES_DIR: &str = ".jules";

/// The `.jlo/` control-plane directory name.
pub const JLO_DIR: &str = ".jlo";

/// The layers directory name.
pub const LAYERS_DIR: &str = "layers";

/// The role definition file name.
pub const ROLE_FILENAME: &str = "role.yml";

/// The scheduled execution file name.
pub const SCHEDULED_FILENAME: &str = "scheduled.toml";

/// The version marker file name.
pub const VERSION_FILE: &str = ".jlo-version";
