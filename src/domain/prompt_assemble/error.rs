/// Error during prompt assembly.
#[derive(Debug, Clone)]
pub enum PromptAssemblyError {
    /// The prompt template file was not found.
    AssemblyTemplateNotFound(String),

    /// Failed to read the prompt template file.
    TemplateReadError { path: String, reason: String },

    /// A required include file was not found.
    RequiredIncludeNotFound { path: String, title: String },

    /// Failed to read an include file.
    IncludeReadError { path: String, reason: String },

    /// Failed to render a template with the provided context.
    TemplateRenderError { template: String, reason: String },

    /// Path traversal detected in include path.
    PathTraversalDetected { path: String },

    /// Failed to seed a missing file from a schema template.
    SchemaSeedError { path: String, reason: String },
}

impl std::fmt::Display for PromptAssemblyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AssemblyTemplateNotFound(path) => {
                write!(f, "Prompt assembly template not found: {}", path)
            }
            Self::TemplateReadError { path, reason } => {
                write!(f, "Failed to read prompt assembly template {}: {}", path, reason)
            }
            Self::RequiredIncludeNotFound { path, title } => {
                write!(f, "Required include '{}' not found: {}", title, path)
            }
            Self::IncludeReadError { path, reason } => {
                write!(f, "Failed to read include {}: {}", path, reason)
            }
            Self::TemplateRenderError { template, reason } => {
                write!(f, "Failed to render template {}: {}", template, reason)
            }
            Self::PathTraversalDetected { path } => {
                write!(f, "Path traversal detected in include path: {}", path)
            }
            Self::SchemaSeedError { path, reason } => {
                write!(f, "Failed to seed include {}: {}", path, reason)
            }
        }
    }
}

impl std::error::Error for PromptAssemblyError {}
