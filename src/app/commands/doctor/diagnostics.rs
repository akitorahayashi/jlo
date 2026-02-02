#[derive(Debug, Clone, Copy)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Diagnostic {
    pub file: String,
    pub message: String,
    pub severity: Severity,
}

#[derive(Debug, Default)]
pub struct Diagnostics {
    errors: Vec<Diagnostic>,
    warnings: Vec<Diagnostic>,
}

impl Diagnostics {
    pub fn push_error(&mut self, file: impl Into<String>, message: impl Into<String>) {
        let diagnostic =
            Diagnostic { file: file.into(), message: message.into(), severity: Severity::Error };
        self.errors.push(diagnostic);
    }

    pub fn push_warning(&mut self, file: impl Into<String>, message: impl Into<String>) {
        let diagnostic =
            Diagnostic { file: file.into(), message: message.into(), severity: Severity::Warning };
        self.warnings.push(diagnostic);
    }

    pub fn error_count(&self) -> usize {
        self.errors.len()
    }

    pub fn warning_count(&self) -> usize {
        self.warnings.len()
    }

    #[allow(dead_code)]
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    #[allow(dead_code)]
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    pub fn emit(&self) {
        for diagnostic in &self.errors {
            eprintln!("[ERROR] {}: {}", diagnostic.file, diagnostic.message);
        }
        for diagnostic in &self.warnings {
            eprintln!("[WARN] {}: {}", diagnostic.file, diagnostic.message);
        }
    }
}
