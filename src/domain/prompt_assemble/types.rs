use std::collections::HashMap;
use std::path::PathBuf;

/// A deferred file-seeding intent produced by prompt assembly.
///
/// The domain computes which files need to be seeded from schema; the
/// application layer executes the actual I/O.
#[derive(Debug, Clone)]
pub struct SeedOp {
    /// Source path (schema file to copy from).
    pub from: PathBuf,
    /// Destination path (target file to seed into).
    pub to: PathBuf,
    /// Whether this seed is required. If `true`, failure to execute must be
    /// propagated as an error.
    pub required: bool,
}

/// Runtime context for prompt assembly.
///
/// Contains the variable values to substitute into include paths.
#[derive(Debug, Clone, Default)]
pub struct PromptContext {
    /// Variable name to value mapping.
    pub variables: HashMap<String, String>,
}

impl PromptContext {
    /// Create a new empty context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a variable to the context.
    pub fn with_var(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.variables.insert(name.into(), value.into());
        self
    }

    /// Get a variable value.
    #[allow(dead_code)]
    pub fn get(&self, name: &str) -> Option<&str> {
        self.variables.get(name).map(|s| s.as_str())
    }
}

/// Result of prompt assembly.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AssembledPrompt {
    /// The fully assembled prompt text.
    pub content: String,

    /// Files that were included (for diagnostic output).
    pub included_files: Vec<String>,

    /// Files that were skipped (optional and missing).
    pub skipped_files: Vec<String>,
}
