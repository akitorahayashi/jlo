use include_dir::{Dir, DirEntry, include_dir};

use crate::domain::Layer;
use crate::ports::{RoleTemplateStore, ScaffoldFile};

static SCAFFOLD_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/assets/scaffold");

/// Role templates for multi-role layers
mod templates {
    pub static OBSERVER_ROLE: &str = include_str!("../assets/templates/layers/observers/role.yml");
    pub static DECIDER_ROLE: &str = include_str!("../assets/templates/layers/deciders/role.yml");
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
        files.sort_by(|a, b| a.path.cmp(&b.path));
        files
    }

    fn layer_template(&self, _layer: Layer) -> &str {
        ""
    }

    fn generate_role_yaml(&self, _role_id: &str, layer: Layer) -> String {
        match layer {
            Layer::Observers => templates::OBSERVER_ROLE.to_string(),
            Layer::Deciders => templates::DECIDER_ROLE.to_string(),
            Layer::Narrator | Layer::Planners | Layer::Implementers => String::new(),
        }
    }
}

fn collect_files(dir: &'static Dir, files: &mut Vec<ScaffoldFile>) {
    for entry in dir.entries() {
        match entry {
            DirEntry::File(file) => {
                if let Some(content) = file.contents_utf8() {
                    let path = file.path().to_string_lossy().to_string();
                    files.push(ScaffoldFile { path, content: content.to_string() });
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
        let store = EmbeddedRoleTemplateStore::new();
        let files = store.scaffold_files();
        assert!(files.iter().any(|f| f.path == ".jules/README.md"));
    }

    #[test]
    fn scaffold_includes_jules_contract() {
        let store = EmbeddedRoleTemplateStore::new();
        let files = store.scaffold_files();
        assert!(files.iter().any(|f| f.path == ".jules/JULES.md"));
    }

    #[test]
    fn generate_role_yaml_for_observers() {
        let store = EmbeddedRoleTemplateStore::new();
        let yaml = store.generate_role_yaml("custom", Layer::Observers);

        assert!(yaml.contains("role: ROLE_NAME"));
        assert!(yaml.contains("layer: observers"));
        assert!(yaml.contains("profile:"));
        assert!(yaml.contains("focus:"));
        assert!(yaml.contains("instructions:"));
    }

    #[test]
    fn generate_role_yaml_for_deciders() {
        let store = EmbeddedRoleTemplateStore::new();
        let yaml = store.generate_role_yaml("custom", Layer::Deciders);

        assert!(yaml.contains("role: ROLE_NAME"));
        assert!(yaml.contains("layer: deciders"));
        assert!(yaml.contains("profile:"));
        assert!(yaml.contains("instructions:"));
    }

    #[test]
    fn generate_role_yaml_empty_for_single_role_layers() {
        let store = EmbeddedRoleTemplateStore::new();

        assert!(store.generate_role_yaml("custom", Layer::Narrator).is_empty());
        assert!(store.generate_role_yaml("custom", Layer::Planners).is_empty());
        assert!(store.generate_role_yaml("custom", Layer::Implementers).is_empty());
    }
}
