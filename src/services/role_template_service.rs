use include_dir::{Dir, DirEntry, include_dir};

use crate::domain::Layer;
use crate::ports::{RoleTemplateStore, ScaffoldFile};

static SCAFFOLD_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/assets/scaffold");

/// Prompt templates for new roles
mod templates {
    pub static ROLE_YML: &str = include_str!("../assets/templates/layers/observer/role.yml");
    pub static OBSERVER: &str = include_str!("../assets/templates/layers/observer/prompt.yml");
    pub static DECIDER: &str = include_str!("../assets/templates/layers/decider/prompt.yml");
    pub static PLANNER: &str = include_str!("../assets/templates/layers/planner/prompt.yml");
    pub static IMPLEMENTER: &str =
        include_str!("../assets/templates/layers/implementer/prompt.yml");
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
        // Only observers have role.yml
        if !matches!(layer, Layer::Observers) {
            return String::new();
        }

        templates::ROLE_YML.to_string()
    }

    fn generate_prompt_yaml_template(&self, _role_id: &str, layer: Layer) -> String {
        // Load the appropriate template and replace ROLE_NAME placeholder
        let template = match layer {
            Layer::Observers => templates::OBSERVER,
            Layer::Deciders => templates::DECIDER,
            Layer::Planners => templates::PLANNER,
            Layer::Implementers => templates::IMPLEMENTER,
        };

        template.to_string()
    }
}

fn collect_files(dir: &'static Dir, files: &mut Vec<ScaffoldFile>) {
    for entry in dir.entries() {
        match entry {
            DirEntry::File(file) => {
                if let Some(content) = file.contents_utf8() {
                    files.push(ScaffoldFile {
                        path: file.path().to_string_lossy().to_string(),
                        content: content.to_string(),
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
    fn generate_role_yaml_has_correct_structure() {
        let store = EmbeddedRoleTemplateStore::new();
        let yaml = store.generate_role_yaml("custom", Layer::Observers);

        assert!(yaml.contains("role: ROLE_NAME"));
        assert!(yaml.contains("focus:"));
        assert!(yaml.contains("notes_strategy:"));
    }

    #[test]
    fn generate_prompt_yaml_template_has_correct_structure() {
        let store = EmbeddedRoleTemplateStore::new();
        let yaml = store.generate_prompt_yaml_template("custom", Layer::Planners);

        assert!(yaml.contains("role: ROLE_NAME"));
    }
}
