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
    pub role_yaml: &'static str,
    /// Optional policy file (only for PM role).
    pub policy: Option<&'static str>,
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

static ROLE_DEFINITIONS: [RoleDefinition; 4] = [
    RoleDefinition {
        id: "taxonomy",
        role_yaml: include_str!("role_kits/taxonomy/role.yml"),
        policy: None,
    },
    RoleDefinition {
        id: "data_arch",
        role_yaml: include_str!("role_kits/data_arch/role.yml"),
        policy: None,
    },
    RoleDefinition { id: "qa", role_yaml: include_str!("role_kits/qa/role.yml"), policy: None },
    RoleDefinition {
        id: "pm",
        role_yaml: include_str!("role_kits/pm/role.yml"),
        policy: Some(include_str!("role_kits/pm/policy.md")),
    },
];

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
    fn scaffold_includes_reports_gitkeep() {
        let files = scaffold_files();
        assert!(files.iter().any(|f| f.path == ".jules/reports/.gitkeep"));
    }

    #[test]
    fn scaffold_includes_issues_structure() {
        let files = scaffold_files();
        assert!(files.iter().any(|f| f.path == ".jules/issues/bugs/.gitkeep"));
        assert!(files.iter().any(|f| f.path == ".jules/issues/refacts/.gitkeep"));
        assert!(files.iter().any(|f| f.path == ".jules/issues/updates/.gitkeep"));
        assert!(files.iter().any(|f| f.path == ".jules/issues/tests/.gitkeep"));
        assert!(files.iter().any(|f| f.path == ".jules/issues/docs/.gitkeep"));
    }

    #[test]
    fn update_managed_files_include_readme() {
        let files = update_managed_files();
        assert!(files.iter().any(|file| file.path == ".jules/README.md"));
    }

    #[test]
    fn role_definitions_includes_all_four_roles() {
        assert_eq!(role_definitions().len(), 4);
        assert!(role_definition("taxonomy").is_some());
        assert!(role_definition("data_arch").is_some());
        assert!(role_definition("qa").is_some());
        assert!(role_definition("pm").is_some());
    }

    #[test]
    fn taxonomy_role_yaml_is_loaded() {
        let taxonomy = role_definition("taxonomy").expect("taxonomy should exist");
        assert!(!taxonomy.role_yaml.is_empty());
        assert!(taxonomy.policy.is_none());
    }

    #[test]
    fn pm_role_has_policy() {
        let pm = role_definition("pm").expect("pm should exist");
        assert!(!pm.role_yaml.is_empty());
        assert!(pm.policy.is_some());
        assert!(!pm.policy.unwrap().is_empty());
    }
}
