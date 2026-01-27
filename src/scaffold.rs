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
    pub prompt_yaml: &'static str,
    pub has_notes: bool,
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
    let mut files: Vec<ScaffoldFile> =
        scaffold_files().into_iter().filter(|file| is_update_managed_path(&file.path)).collect();

    for role in role_definitions() {
        files.push(ScaffoldFile {
            path: format!(".jules/roles/{}/prompt.yml", role.id),
            content: role.prompt_yaml,
        });
    }

    files.sort_by(|a, b| a.path.cmp(&b.path));
    files
}

/// Returns all built-in role definitions.
pub fn role_definitions() -> &'static [RoleDefinition] {
    &ROLE_DEFINITIONS
}

/// Lookup a built-in role definition by id.
pub fn role_definition(role_id: &str) -> Option<&'static RoleDefinition> {
    ROLE_DEFINITIONS.iter().find(|role| role.id == role_id)
}

static ROLE_DEFINITIONS: [RoleDefinition; 6] = [
    RoleDefinition {
        id: "taxonomy",
        role_yaml: include_str!("role_kits/taxonomy/role.yml"),
        prompt_yaml: include_str!("role_kits/taxonomy/prompt.yml"),
        has_notes: true,
    },
    RoleDefinition {
        id: "data_arch",
        role_yaml: include_str!("role_kits/data_arch/role.yml"),
        prompt_yaml: include_str!("role_kits/data_arch/prompt.yml"),
        has_notes: true,
    },
    RoleDefinition {
        id: "qa",
        role_yaml: include_str!("role_kits/qa/role.yml"),
        prompt_yaml: include_str!("role_kits/qa/prompt.yml"),
        has_notes: true,
    },
    RoleDefinition {
        id: "triage",
        role_yaml: include_str!("role_kits/triage/role.yml"),
        prompt_yaml: include_str!("role_kits/triage/prompt.yml"),
        has_notes: false,
    },
    RoleDefinition {
        id: "specifier",
        role_yaml: include_str!("role_kits/specifier/role.yml"),
        prompt_yaml: include_str!("role_kits/specifier/prompt.yml"),
        has_notes: false,
    },
    RoleDefinition {
        id: "executor",
        role_yaml: include_str!("role_kits/executor/role.yml"),
        prompt_yaml: include_str!("role_kits/executor/prompt.yml"),
        has_notes: false,
    },
];

/// Check if a path is managed by `jo update`.
pub fn is_update_managed_path(path: &str) -> bool {
    matches!(path, ".jules/README.md" | ".jules/JULES.md")
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
    fn scaffold_includes_jules_contract() {
        let files = scaffold_files();
        assert!(files.iter().any(|f| f.path == ".jules/JULES.md"));
    }

    #[test]
    fn scaffold_includes_issues_structure() {
        let files = scaffold_files();
        assert!(files.iter().any(|f| f.path == ".jules/issues/.gitkeep"));
        assert!(files.iter().any(|f| f.path == ".jules/tasks/.gitkeep"));
        assert!(files.iter().any(|f| f.path == ".jules/events/bugs/.gitkeep"));
        assert!(files.iter().any(|f| f.path == ".jules/events/docs/.gitkeep"));
        assert!(files.iter().any(|f| f.path == ".jules/events/refacts/.gitkeep"));
        assert!(files.iter().any(|f| f.path == ".jules/events/tests/.gitkeep"));
        assert!(files.iter().any(|f| f.path == ".jules/events/updates/.gitkeep"));
    }

    #[test]
    fn update_managed_files_include_readme() {
        let files = update_managed_files();
        assert!(files.iter().any(|file| file.path == ".jules/README.md"));
        assert!(files.iter().any(|file| file.path == ".jules/JULES.md"));
        assert!(files.iter().any(|file| file.path == ".jules/roles/taxonomy/prompt.yml"));
    }

    #[test]
    fn role_definitions_includes_all_six_roles() {
        use std::collections::HashSet;
        let expected_ids: HashSet<&str> =
            ["taxonomy", "data_arch", "qa", "triage", "specifier", "executor"]
                .iter()
                .cloned()
                .collect();
        let actual_ids: HashSet<&str> = role_definitions().iter().map(|r| r.id).collect();
        assert_eq!(actual_ids, expected_ids);
    }

    #[test]
    fn taxonomy_role_yaml_is_loaded() {
        let taxonomy = role_definition("taxonomy").expect("taxonomy should exist");
        assert!(!taxonomy.role_yaml.is_empty());
        assert!(!taxonomy.prompt_yaml.is_empty());
    }

    #[test]
    fn triage_role_has_prompt() {
        let triage = role_definition("triage").expect("triage should exist");
        assert!(!triage.role_yaml.is_empty());
        assert!(!triage.prompt_yaml.is_empty());
    }
}
