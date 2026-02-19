use std::fs;
use std::path::Path;

use crate::app::commands::doctor::diagnostics::Diagnostics;
use crate::domain::workstations::perspectives::DATETIME_PLACEHOLDER;

pub fn check_placeholders_file(path: &Path, diagnostics: &mut Diagnostics) {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(err) => {
            diagnostics
                .push_error(path.display().to_string(), format!("Failed to read file: {}", err));
            return;
        }
    };
    check_placeholders(&content, path, diagnostics);
}

pub fn check_placeholders(content: &str, path: &Path, diagnostics: &mut Diagnostics) {
    let placeholders = [
        "<6_random_lowercase_alphanumeric_chars>",
        "<role>",
        "<Descriptive Title>",
        DATETIME_PLACEHOLDER,
        "<path>",
        "<condition 1>",
        "<condition 2>",
    ];

    for placeholder in placeholders {
        if content.contains(placeholder) {
            diagnostics.push_error(
                path.display().to_string(),
                format!("placeholder '{}' must be replaced", placeholder),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_check_placeholders_content() {
        let content = "This is a <role> description.";
        let path = PathBuf::from("test.yml");
        let mut diagnostics = Diagnostics::default();

        check_placeholders(content, &path, &mut diagnostics);
        assert_eq!(diagnostics.error_count(), 1);
        assert!(diagnostics.errors()[0].message.contains("placeholder '<role>' must be replaced"));
    }

    #[test]
    fn test_check_placeholders_datetime() {
        let content = "updated_at: YYYY-MM-DD";
        let path = PathBuf::from("test.yml");
        let mut diagnostics = Diagnostics::default();

        check_placeholders(content, &path, &mut diagnostics);
        assert!(diagnostics.error_count() > 0);
        let messages: Vec<_> = diagnostics.errors().iter().map(|e| &e.message).collect();
        assert!(messages.iter().any(|m| m.contains("placeholder 'YYYY-MM-DD' must be replaced")));
    }
}
