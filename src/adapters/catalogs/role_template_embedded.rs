use include_dir::{Dir, DirEntry, include_dir};

use crate::adapters::catalogs::builtin_role_assets::{
    load_builtin_role_catalog, read_builtin_role_file,
};
use crate::domain::{AppError, BuiltinRoleEntry, Layer};
use crate::ports::{RoleTemplateStore, ScaffoldFile};

static SCAFFOLD_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/assets/scaffold");

/// Internal documentation file that should not be deployed to user scaffolds.
const INTERNAL_DOC_FILE: &str = "AGENTS.md";

/// Role templates for multi-role layers
mod templates {
    pub static OBSERVER_ROLE: &str =
        include_str!("../../assets/templates/layers/observers/role.yml");
    pub static INNOVATOR_ROLE: &str =
        include_str!("../../assets/templates/layers/innovators/role.yml");
}

/// Embedded role template store implementation.
#[derive(Debug, Clone, Default)]
pub struct EmbeddedRoleTemplateStore;

impl EmbeddedRoleTemplateStore {
    pub fn new() -> Self {
        Self
    }
}

impl RoleTemplateStore for EmbeddedRoleTemplateStore {
    fn scaffold_files(&self) -> Vec<ScaffoldFile> {
        let mut files = Vec::new();
        collect_files(&SCAFFOLD_DIR, &mut files);
        // Return only .jules/ scaffold files (not .jlo/)
        files.retain(|f| f.path.starts_with(".jules/"));
        files.sort_by(|a, b| a.path.cmp(&b.path));
        files
    }

    fn control_plane_files(&self) -> Vec<ScaffoldFile> {
        let mut files = Vec::new();
        collect_files(&SCAFFOLD_DIR, &mut files);
        files.retain(|f| f.path.starts_with(".jlo/"));
        files.sort_by(|a, b| a.path.cmp(&b.path));
        files
    }

    fn control_plane_skeleton_files(&self) -> Vec<ScaffoldFile> {
        let all = self.control_plane_files();
        all.into_iter().filter(|f| !is_entity_file(&f.path)).collect()
    }

    fn layer_template(&self, _layer: Layer) -> &str {
        ""
    }

    fn generate_role_yaml(&self, _role_id: &str, layer: Layer) -> String {
        match layer {
            Layer::Observers => templates::OBSERVER_ROLE.to_string(),
            Layer::Innovators => templates::INNOVATOR_ROLE.to_string(),
            Layer::Decider
            | Layer::Narrator
            | Layer::Planner
            | Layer::Implementer
            | Layer::Integrator => String::new(),
        }
    }

    fn builtin_role_catalog(&self) -> Result<Vec<BuiltinRoleEntry>, AppError> {
        load_builtin_role_catalog()
    }

    fn builtin_role_content(&self, layer: Layer, role_id: &str) -> Result<String, AppError> {
        let entry = load_builtin_role_catalog()?
            .into_iter()
            .find(|entry| entry.layer == layer && entry.name.as_str() == role_id)
            .ok_or_else(|| {
                AppError::Validation(format!(
                    "Builtin role '{}' not found in layer '{}'",
                    role_id,
                    layer.dir_name()
                ))
            })?;
        read_builtin_role_file(&entry.path)
    }
}

fn collect_files(dir: &'static Dir, files: &mut Vec<ScaffoldFile>) {
    for entry in dir.entries() {
        match entry {
            DirEntry::File(file) => {
                if let Some(content) = file.contents_utf8() {
                    let raw_path = file.path().to_string_lossy().to_string();
                    // Don't include the internal documentation in the deployed scaffold
                    if std::path::Path::new(&raw_path).file_name()
                        != Some(std::ffi::OsStr::new(INTERNAL_DOC_FILE))
                    {
                        let path = map_scaffold_path(&raw_path);
                        files.push(ScaffoldFile { path, content: content.to_string() });
                    }
                }
            }
            DirEntry::Dir(subdir) => collect_files(subdir, files),
        }
    }
}

/// Map embedded scaffold paths to deployment paths.
///
/// Source directories use plain names (`jlo/`, `jules/`) so they are visible
/// to search tools like ripgrep and ag which skip hidden directories by default.
/// Deployment targets use dot-prefixed names (`.jlo/`, `.jules/`).
///
/// Additionally, `jules/layers/` maps to `.jules/layers/` (implicitly via `jules/` prefix).
fn map_scaffold_path(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("jlo/") {
        format!(".jlo/{}", rest)
    } else if let Some(rest) = path.strip_prefix("jules/") {
        format!(".jules/{}", rest)
    } else {
        path.to_string()
    }
}

/// Returns true for user-authored entity files.
/// These files should not be recreated by `update` if deleted.
fn is_entity_file(path: &str) -> bool {
    // Role file: .jlo/roles/<layer>/<role>/role.yml or .jlo/roles/<layer>/role.yml (single-role)
    path.ends_with("/role.yml") && path.starts_with(".jlo/roles/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaffold_excludes_jlo_files() {
        let store = EmbeddedRoleTemplateStore::new();
        let files = store.scaffold_files();
        assert!(files.iter().all(|f| f.path.starts_with(".jules/")));
    }

    #[test]
    fn control_plane_files_include_config() {
        let store = EmbeddedRoleTemplateStore::new();
        let files = store.control_plane_files();
        assert!(files.iter().any(|f| f.path == ".jlo/config.toml"));
    }

    #[test]
    fn control_plane_files_include_setup() {
        let store = EmbeddedRoleTemplateStore::new();
        let files = store.control_plane_files();
        assert!(files.iter().any(|f| f.path == ".jlo/setup/tools.yml"));
        assert!(files.iter().any(|f| f.path == ".jlo/setup/.gitignore"));
    }

    #[test]
    fn control_plane_files_exclude_decider_role_customizations() {
        let store = EmbeddedRoleTemplateStore::new();
        let files = store.control_plane_files();
        assert!(files.iter().all(|f| f.path != ".jlo/roles/decider/role.yml"));
    }

    #[test]
    fn control_plane_files_exclude_managed_framework() {
        let store = EmbeddedRoleTemplateStore::new();
        let files = store.control_plane_files();
        // Managed framework files should not be in control plane
        assert!(files.iter().all(|f| !f.path.ends_with("/contracts.yml")));
        assert!(files.iter().all(|f| !f.path.ends_with("/prompt.yml")));
        assert!(files.iter().all(|f| !f.path.contains("/schemas/")));
        assert!(files.iter().all(|f| f.path != ".jlo/JULES.md"));
        assert!(files.iter().all(|f| f.path != ".jlo/README.md"));
    }

    #[test]
    fn skeleton_files_exclude_entities() {
        let store = EmbeddedRoleTemplateStore::new();
        let skeleton = store.control_plane_skeleton_files();
        // Skeleton must not contain role definitions
        assert!(skeleton.iter().all(|f| !f.path.ends_with("/role.yml")));
        // But must contain infrastructure
        assert!(skeleton.iter().any(|f| f.path == ".jlo/config.toml"));
        assert!(skeleton.iter().any(|f| f.path == ".jlo/setup/tools.yml"));
    }

    #[test]
    fn generate_role_yaml_for_observers() {
        let store = EmbeddedRoleTemplateStore::new();
        let yaml = store.generate_role_yaml("custom", Layer::Observers);

        assert!(yaml.contains("role: ROLE_NAME"));
        assert!(yaml.contains("layer: observers"));
        assert!(yaml.contains("profile:"));
        assert!(yaml.contains("focus:"));
    }

    #[test]
    fn generate_role_yaml_for_innovators() {
        let store = EmbeddedRoleTemplateStore::new();
        let yaml = store.generate_role_yaml("custom", Layer::Innovators);

        assert!(yaml.contains("role: ROLE_NAME"));
        assert!(yaml.contains("layer: innovators"));
        assert!(yaml.contains("profile:"));
        assert!(yaml.contains("focus:"));
    }

    #[test]
    fn generate_role_yaml_empty_for_single_role_layers() {
        let store = EmbeddedRoleTemplateStore::new();

        assert!(store.generate_role_yaml("custom", Layer::Decider).is_empty());
        assert!(store.generate_role_yaml("custom", Layer::Narrator).is_empty());
        assert!(store.generate_role_yaml("custom", Layer::Planner).is_empty());
        assert!(store.generate_role_yaml("custom", Layer::Implementer).is_empty());
    }
}
