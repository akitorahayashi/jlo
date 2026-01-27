use include_dir::{Dir, DirEntry, include_dir};

use crate::domain::Layer;
use crate::ports::{RoleTemplateStore, ScaffoldFile};

static SCAFFOLD_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/assets/scaffold");

/// Prompt templates for new roles
mod prompt_templates {
    pub static OBSERVER: &str = include_str!("../assets/archetypes/layers/observer/template.yml");
    pub static DECIDER: &str = include_str!("../assets/archetypes/layers/decider/template.yml");
    pub static PLANNER: &str = include_str!("../assets/archetypes/layers/planner/template.yml");
    pub static IMPLEMENTER: &str =
        include_str!("../assets/archetypes/layers/implementer/template.yml");
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
        // Archetypes are not used at runtime; this is kept for compatibility
        ""
    }

    fn generate_role_yaml(&self, role_id: &str, layer: Layer) -> String {
        // Only observers have role.yml
        if !matches!(layer, Layer::Observers) {
            return String::new();
        }

        format!(
            r#"role: {role_id}

focus: |
  # TODO: Describe the specialized analytical focus for this observer

notes_strategy: |
  # TODO: Describe how to organize and update notes/

feedback_integration: |
  At the start of each execution, read all files in feedbacks/.
  Abstract common patterns and refine focus to reduce noise.

learned_exclusions: []
"#
        )
    }

    fn generate_prompt_yaml_template(&self, role_id: &str, layer: Layer) -> String {
        // Load the appropriate template and replace ROLE_NAME placeholder
        let template = match layer {
            Layer::Observers => prompt_templates::OBSERVER,
            Layer::Deciders => prompt_templates::DECIDER,
            Layer::Planners => prompt_templates::PLANNER,
            Layer::Implementers => prompt_templates::IMPLEMENTER,
        };

        template.replace("ROLE_NAME", role_id)
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

        assert!(yaml.contains("role: custom"));
        assert!(yaml.contains("focus:"));
        assert!(yaml.contains("notes_strategy:"));
    }

    #[test]
    fn generate_prompt_yaml_template_has_correct_structure() {
        let store = EmbeddedRoleTemplateStore::new();
        let yaml = store.generate_prompt_yaml_template("custom", Layer::Planners);

        assert!(yaml.contains("role: custom"));
        assert!(yaml.contains("target_issue:"));
        assert!(yaml.contains("prompt:"));
        assert!(yaml.contains("Read JULES.md"));
    }
}
