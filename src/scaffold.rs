//! Embedded scaffold content for `.jules/` deployment.

use std::collections::HashMap;
use std::sync::LazyLock;

use include_dir::{Dir, DirEntry, include_dir};

static SCAFFOLD_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/scaffold");
static ROLE_KITS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/role_kits");

/// A file embedded in the scaffold bundle.
#[derive(Debug, Clone)]
pub struct ScaffoldFile {
    /// Path relative to the scaffold root.
    pub path: String,
    /// File content as UTF-8 text.
    pub content: &'static str,
}

/// Definition of a built-in role template.
#[derive(Debug, Clone)]
pub struct RoleDefinition {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub charter: &'static str,
    pub direction: &'static str,
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

/// Lookup a scaffold file by path.
pub fn file_content(path: &str) -> Option<&'static str> {
    SCAFFOLD_DIR.get_file(path)?.contents_utf8()
}

/// Load a template file from `.jules/.jo/templates/`.
pub fn template_content(name: &str) -> Option<&'static str> {
    let path = format!(".jules/.jo/templates/{}", name);
    file_content(&path)
}

/// Returns all built-in role definitions.
pub fn role_definitions() -> &'static [RoleDefinition] {
    &ROLE_DEFINITIONS
}

/// Lookup a built-in role definition by id.
pub fn role_definition(role_id: &str) -> Option<&'static RoleDefinition> {
    ROLE_DEFINITIONS.iter().find(|role| role.id == role_id)
}

static ROLE_DEFINITIONS: LazyLock<Vec<RoleDefinition>> = LazyLock::new(|| {
    let mut roles = Vec::new();
    let mut role_paths = Vec::new();
    collect_file_paths(&ROLE_KITS_DIR, &mut role_paths);

    for path in role_paths {
        if !path.ends_with("meta.txt") {
            continue;
        }

        let content = match ROLE_KITS_DIR.get_file(&path).and_then(|file| file.contents_utf8()) {
            Some(content) => content,
            None => continue,
        };

        let metadata = parse_meta(content);
        let id = metadata.get("id").cloned().unwrap_or_else(|| role_id_from_path(&path));
        if id.is_empty() {
            continue;
        }

        let base_dir = path.trim_end_matches("meta.txt");
        let charter_path = format!("{}charter.md", base_dir);
        let direction_path = format!("{}direction.md", base_dir);

        let Some(charter) = ROLE_KITS_DIR.get_file(&charter_path).and_then(|f| f.contents_utf8())
        else {
            continue;
        };
        let Some(direction) =
            ROLE_KITS_DIR.get_file(&direction_path).and_then(|f| f.contents_utf8())
        else {
            continue;
        };

        roles.push(RoleDefinition {
            id,
            title: metadata.get("title").cloned().unwrap_or_default(),
            summary: metadata.get("summary").cloned().unwrap_or_default(),
            charter,
            direction,
        });
    }

    roles.sort_by(|a, b| a.id.cmp(&b.id));
    roles
});

fn parse_meta(content: &str) -> HashMap<String, String> {
    content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| (key.trim().to_string(), value.trim().to_string()))
        .collect()
}

fn role_id_from_path(path: &str) -> String {
    std::path::Path::new(path)
        .parent()
        .and_then(|parent| parent.as_os_str().to_str())
        .unwrap_or("")
        .to_string()
}

fn is_update_managed_path(path: &str) -> bool {
    path.starts_with(".jules/.jo/") || path == ".jules/README.md" || path.ends_with("/.gitkeep")
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

fn collect_file_paths(dir: &'static Dir, paths: &mut Vec<String>) {
    for entry in dir.entries() {
        match entry {
            DirEntry::File(file) => {
                paths.push(file.path().to_string_lossy().to_string());
            }
            DirEntry::Dir(subdir) => collect_file_paths(subdir, paths),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaffold_includes_policy_files() {
        assert!(file_content(".jules/.jo/policy/contract.md").is_some());
    }

    #[test]
    fn scaffold_files_include_policy_paths() {
        let files = scaffold_files();
        assert!(files.iter().any(|file| file.path == ".jules/.jo/policy/contract.md"));
    }

    #[test]
    fn update_managed_files_include_readme() {
        let files = update_managed_files();
        assert!(files.iter().any(|file| file.path == ".jules/README.md"));
    }

    #[test]
    fn role_kits_are_loaded() {
        assert!(!role_definitions().is_empty());
    }
}
