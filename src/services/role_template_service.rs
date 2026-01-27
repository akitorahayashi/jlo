use include_dir::{Dir, DirEntry, include_dir};

use crate::domain::Layer;
use crate::ports::{RoleTemplateStore, ScaffoldFile};

static SCAFFOLD_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/assets/scaffold");

/// Layer-specific templates.
mod layer_templates {
    pub static OBSERVER: &str =
        include_str!("../assets/scaffold/.jules/archetypes/layers/observer.yml");
    pub static DECIDER: &str =
        include_str!("../assets/scaffold/.jules/archetypes/layers/decider.yml");
    pub static PLANNER: &str =
        include_str!("../assets/scaffold/.jules/archetypes/layers/planner.yml");
    pub static IMPLEMENTER: &str =
        include_str!("../assets/scaffold/.jules/archetypes/layers/implementer.yml");
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

    fn layer_template(&self, layer: Layer) -> &str {
        match layer {
            Layer::Observers => layer_templates::OBSERVER,
            Layer::Deciders => layer_templates::DECIDER,
            Layer::Planners => layer_templates::PLANNER,
            Layer::Implementers => layer_templates::IMPLEMENTER,
        }
    }

    fn generate_role_yaml(&self, role_id: &str, layer: Layer) -> String {
        let description = layer.description();
        let layer_dir = layer.dir_name();
        let layer_file = layer_dir.trim_end_matches('s');
        let role_type = match layer {
            Layer::Observers => "worker",
            Layer::Deciders => "manager",
            Layer::Planners => "planner",
            Layer::Implementers => "implementer",
        };

        format!(
            r#"role: {role_id}
layer: {layer_dir}
type: {role_type}

goal: |
  # TODO: このロールの目的を記述してください

# アーキタイプ参照:
# {description}
# 詳細は .jules/archetypes/layers/{layer_file}.yml を参照
"#
        )
    }

    fn generate_prompt_yaml_template(&self, role_id: &str, layer: Layer) -> String {
        let layer_name = layer.dir_name();

        format!(
            r#"role: {role_id}
layer: {layer_name}
prompt: |
  あなたは {role_id} です（{layer_name} レイヤー）。
  必ず以下を読み、最新の制約に従って行動してください。
  - JULES.md
  - .jules/JULES.md
  - .jules/roles/{layer_name}/{role_id}/role.yml
"#
        )
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
    fn all_layer_templates_exist() {
        let store = EmbeddedRoleTemplateStore::new();
        for layer in Layer::ALL {
            let template = store.layer_template(layer);
            assert!(!template.is_empty(), "Template for {:?} should not be empty", layer);
        }
    }

    #[test]
    fn scaffold_includes_archetypes() {
        let store = EmbeddedRoleTemplateStore::new();
        let files = store.scaffold_files();
        let expected_paths = [
            ".jules/archetypes/layers/observer.yml",
            ".jules/archetypes/layers/decider.yml",
            ".jules/archetypes/layers/planner.yml",
            ".jules/archetypes/layers/implementer.yml",
        ];

        for path in expected_paths {
            assert!(files.iter().any(|f| f.path == path), "Missing scaffold file: {}", path);
        }
    }

    #[test]
    fn generate_role_yaml_has_correct_structure() {
        let store = EmbeddedRoleTemplateStore::new();
        let yaml = store.generate_role_yaml("custom", Layer::Observers);

        assert!(yaml.contains("role: custom"));
        assert!(yaml.contains("layer: observers"));
        assert!(yaml.contains("type: worker"));
    }

    #[test]
    fn generate_prompt_yaml_template_has_correct_structure() {
        let store = EmbeddedRoleTemplateStore::new();
        let yaml = store.generate_prompt_yaml_template("custom", Layer::Planners);

        assert!(yaml.contains("role: custom"));
        assert!(yaml.contains("layer: planners"));
        assert!(yaml.contains("prompt:"));
    }
}
