use include_dir::{Dir, DirEntry, include_dir};

use crate::domain::Layer;
use crate::ports::{RoleDefinition, RoleTemplateStore, ScaffoldFile};

static SCAFFOLD_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/assets/scaffold");

static ROLE_DEFINITIONS: [RoleDefinition; 6] = [
    RoleDefinition {
        id: "taxonomy",
        layer: Layer::Observers,
        role_yaml: include_str!("../assets/role_kits/taxonomy/role.yml"),
        prompt_yaml: include_str!("../assets/role_kits/taxonomy/prompt.yml"),
        has_notes: true,
    },
    RoleDefinition {
        id: "data_arch",
        layer: Layer::Observers,
        role_yaml: include_str!("../assets/role_kits/data_arch/role.yml"),
        prompt_yaml: include_str!("../assets/role_kits/data_arch/prompt.yml"),
        has_notes: true,
    },
    RoleDefinition {
        id: "qa",
        layer: Layer::Observers,
        role_yaml: include_str!("../assets/role_kits/qa/role.yml"),
        prompt_yaml: include_str!("../assets/role_kits/qa/prompt.yml"),
        has_notes: true,
    },
    RoleDefinition {
        id: "triage",
        layer: Layer::Deciders,
        role_yaml: include_str!("../assets/role_kits/triage/role.yml"),
        prompt_yaml: include_str!("../assets/role_kits/triage/prompt.yml"),
        has_notes: false,
    },
    RoleDefinition {
        id: "specifier",
        layer: Layer::Planners,
        role_yaml: include_str!("../assets/role_kits/specifier/role.yml"),
        prompt_yaml: include_str!("../assets/role_kits/specifier/prompt.yml"),
        has_notes: false,
    },
    RoleDefinition {
        id: "executor",
        layer: Layer::Implementers,
        role_yaml: include_str!("../assets/role_kits/executor/role.yml"),
        prompt_yaml: include_str!("../assets/role_kits/executor/prompt.yml"),
        has_notes: false,
    },
];

/// Layer-specific templates.
mod layer_templates {
    pub static OBSERVER: &str = include_str!("../assets/templates/layers/observer.yml");
    pub static DECIDER: &str = include_str!("../assets/templates/layers/decider.yml");
    pub static PLANNER: &str = include_str!("../assets/templates/layers/planner.yml");
    pub static IMPLEMENTER: &str = include_str!("../assets/templates/layers/implementer.yml");
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

    fn role_definitions(&self) -> &[RoleDefinition] {
        &ROLE_DEFINITIONS
    }

    fn role_definition(&self, role_id: &str) -> Option<&RoleDefinition> {
        ROLE_DEFINITIONS.iter().find(|role| role.id == role_id)
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
# 詳細は src/templates/layers/{layer_file}.yml を参照
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
    fn role_definitions_includes_all_six_roles() {
        use std::collections::HashSet;
        let store = EmbeddedRoleTemplateStore::new();
        let expected_ids: HashSet<&str> =
            ["taxonomy", "data_arch", "qa", "triage", "specifier", "executor"]
                .iter()
                .cloned()
                .collect();
        let actual_ids: HashSet<&str> = store.role_definitions().iter().map(|r| r.id).collect();
        assert_eq!(actual_ids, expected_ids);
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
