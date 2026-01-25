//! Embedded scaffold content for `.jules/` deployment.

use include_dir::{Dir, DirEntry, include_dir};

static SCAFFOLD_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/scaffold");

/// A file embedded in the scaffold bundle.
#[derive(Debug, Clone)]
pub struct ScaffoldFile {
    /// Path relative to the scaffold root.
    pub path: String,
    /// File content as UTF-8 text.
    pub content: &'static str,
}

/// Definition of a built-in role.
#[derive(Debug, Clone)]
pub struct RoleDefinition {
    pub id: &'static str,
    pub prompt: &'static str,
}

/// Returns all scaffold files (relative to `src/scaffold/`).
pub fn scaffold_files() -> Vec<ScaffoldFile> {
    let mut files = Vec::new();
    collect_files(&SCAFFOLD_DIR, &mut files);

    files.sort_by(|a, b| a.path.cmp(&b.path));
    files
}

/// Returns scaffold files that `jo update` may overwrite.
pub fn update_managed_files() -> Vec<ScaffoldFile> {
    scaffold_files().into_iter().filter(|file| is_update_managed_path(&file.path)).collect()
}

/// Returns all built-in role definitions.
pub fn role_definitions() -> &'static [RoleDefinition] {
    &ROLE_DEFINITIONS
}

/// Lookup a built-in role definition by id.
pub fn role_definition(role_id: &str) -> Option<&'static RoleDefinition> {
    ROLE_DEFINITIONS.iter().find(|role| role.id == role_id)
}

static ROLE_DEFINITIONS: [RoleDefinition; 1] =
    [RoleDefinition { id: "taxonomy", prompt: include_str!("role_kits/taxonomy/prompt.yml") }];

/// Check if a path is managed by `jo update`.
pub fn is_update_managed_path(path: &str) -> bool {
    matches!(path, ".jules/README.md" | ".jules/.jo-version")
}

fn collect_files(dir: &'static Dir, files: &mut Vec<ScaffoldFile>) {
    for entry in dir.entries() {
        match entry {
            DirEntry::File(file) => {
                if let Some(content) = file.contents_utf8() {
                    files.push(ScaffoldFile {
                        path: file.path().to_string_lossy().to_string(),
                        content,
                    });
                }
            }
            DirEntry::Dir(subdir) => collect_files(subdir, files),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaffold_includes_readme() {
        let files = scaffold_files();
        assert!(files.iter().any(|f| f.path == ".jules/README.md"));
    }

    #[test]
    fn update_managed_files_include_readme() {
        let files = update_managed_files();
        assert!(files.iter().any(|file| file.path == ".jules/README.md"));
    }

    #[test]
    fn role_definitions_includes_taxonomy() {
        assert_eq!(role_definitions().len(), 1);
        assert_eq!(role_definitions()[0].id, "taxonomy");
    }

    #[test]
    fn taxonomy_prompt_is_loaded() {
        let taxonomy = role_definition("taxonomy").expect("taxonomy should exist");
        assert!(!taxonomy.prompt.is_empty());
    }
}
