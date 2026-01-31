use include_dir::{Dir, DirEntry, include_dir};

use crate::domain::Layer;
use crate::ports::{RoleTemplateStore, ScaffoldFile};

static SCAFFOLD_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/assets/scaffold");

/// Prompt templates for new roles (multi-role layers only)
mod templates {
    pub static ROLE_YML: &str = include_str!("../assets/templates/layers/observers/role.yml");
    pub static OBSERVER: &str = include_str!("../assets/templates/layers/observers/prompt.yml");
    pub static DECIDER: &str = include_str!("../assets/templates/layers/deciders/prompt.yml");
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
        // Single-role layers (Planners, Implementers) don't support template creation
        // Return the template as-is with placeholders for multi-role layers
        let template = match layer {
            Layer::Observers => templates::OBSERVER,
            Layer::Deciders => templates::DECIDER,
            Layer::Planners | Layer::Implementers => "",
        };

        template.to_string()
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
    fn generate_role_yaml_has_correct_structure() {
        let store = EmbeddedRoleTemplateStore::new();
        let yaml = store.generate_role_yaml("custom", Layer::Observers);

        assert!(yaml.contains("role: ROLE_NAME"));
        assert!(yaml.contains("focus:"));
        assert!(yaml.contains("learned_exclusions:"));
    }

    #[test]
    fn generate_prompt_yaml_template_has_correct_structure() {
        let store = EmbeddedRoleTemplateStore::new();

        // Test with observer (multi-role layer)
        let yaml = store.generate_prompt_yaml_template("custom", Layer::Observers);
        assert!(yaml.contains("role: ROLE_NAME"));
        assert!(yaml.contains("layer: observers"));
        assert!(yaml.contains("responsibility:"));
        assert!(yaml.contains("contracts:"));
        assert!(yaml.contains("instructions:"));

        // Test with decider (multi-role layer)
        let yaml = store.generate_prompt_yaml_template("custom", Layer::Deciders);
        assert!(yaml.contains("role: ROLE_NAME"));
        assert!(yaml.contains("layer: deciders"));

        // Single-role layers do not provide templates
        let yaml = store.generate_prompt_yaml_template("custom", Layer::Planners);
        assert!(yaml.is_empty());
        let yaml = store.generate_prompt_yaml_template("custom", Layer::Implementers);
        assert!(yaml.is_empty());
    }
}
