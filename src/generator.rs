//! Prompt generator for composing structured YAML prompts.

use serde::{Deserialize, Serialize};

use crate::layers::Layer;

/// A structured prompt ready for clipboard transport.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedPrompt {
    /// Role identifier.
    pub role: String,
    /// Layer this role belongs to.
    pub layer: String,
    /// Assigned context paths (user-provided).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assign: Vec<String>,
    /// The prompt instruction text.
    pub prompt: String,
}

/// Builder for generating prompts.
#[derive(Debug)]
pub struct PromptBuilder {
    role_id: String,
    layer: Layer,
    paths: Vec<String>,
}

impl PromptBuilder {
    /// Create a new prompt builder for a role.
    pub fn new(role_id: impl Into<String>, layer: Layer) -> Self {
        Self { role_id: role_id.into(), layer, paths: Vec::new() }
    }

    /// Add context paths to the prompt.
    pub fn with_paths(mut self, paths: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.paths.extend(paths.into_iter().map(|p| p.into()));
        self
    }

    /// Build the prompt structure.
    pub fn build(self) -> GeneratedPrompt {
        let prompt_text = self.generate_prompt_text();

        GeneratedPrompt {
            role: self.role_id,
            layer: self.layer.dir_name().to_string(),
            assign: self.paths,
            prompt: prompt_text,
        }
    }

    /// Generate the prompt instruction text.
    fn generate_prompt_text(&self) -> String {
        let layer_name = self.layer.dir_name();
        let role_id = &self.role_id;

        format!(
            r#"あなたは {role_id} です（{layer} レイヤー）。
必ず以下を読み、最新の制約に従って行動してください。
- JULES.md
- .jules/JULES.md
- .jules/roles/{layer}/{role_id}/role.yml

{layer_behavior}"#,
            role_id = role_id,
            layer = layer_name,
            layer_behavior = self.layer_specific_behavior()
        )
    }

    /// Get layer-specific behavior instructions.
    fn layer_specific_behavior(&self) -> &'static str {
        match self.layer {
            Layer::Observers => {
                "役割に沿って観測し、events/ にYAMLで記録し、notes/ を宣言的に更新する。\n\
                 issues/ には書き込まない。プロダクトコードは編集しない。"
            }
            Layer::Deciders => {
                "events/ を批判的に評価し、採用/却下を判断する。\n\
                 採用した観測を issues/ にMarkdownで作成する。\n\
                 処理済みの events/ は削除する。プロダクトコードは編集しない。"
            }
            Layer::Planners => {
                "issues/ を読み込み、具体的なタスクに分解する。\n\
                 tasks/ に検証計画を含むタスクファイルを作成する。\n\
                 処理済みの issues/ は削除する。プロダクトコードは編集しない。"
            }
            Layer::Implementers => {
                "tasks/ を読み込み、コードを実装する。\n\
                 検証を実行し、成功したら該当 tasks/ を削除する。\n\
                 レポートファイルは生成しない。成果はコード変更のみ。"
            }
        }
    }

    /// Serialize the built prompt to YAML, consuming the builder.
    pub fn into_yaml(self) -> Result<String, serde_yaml::Error> {
        let prompt = self.build();
        serde_yaml::to_string(&prompt)
    }
}

/// Generate a prompt for a role and serialize to YAML.
pub fn generate_prompt_yaml(
    role_id: &str,
    layer: Layer,
    paths: &[String],
) -> Result<String, serde_yaml::Error> {
    PromptBuilder::new(role_id, layer).with_paths(paths.iter().cloned()).into_yaml()
}

/// Generate a role.yml template for a new custom role.
pub fn generate_role_yaml(role_id: &str, layer: Layer) -> String {
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

/// Generate a prompt.yml template for a new custom role.
pub fn generate_prompt_yaml_template(role_id: &str, layer: Layer) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_builder_creates_valid_structure() {
        let prompt = PromptBuilder::new("taxonomy", Layer::Observers).build();

        assert_eq!(prompt.role, "taxonomy");
        assert_eq!(prompt.layer, "observers");
        assert!(prompt.assign.is_empty());
        assert!(!prompt.prompt.is_empty());
    }

    #[test]
    fn prompt_builder_injects_paths() {
        let prompt = PromptBuilder::new("taxonomy", Layer::Observers)
            .with_paths(["src/main.rs", "src/lib.rs"])
            .build();

        assert_eq!(prompt.assign.len(), 2);
        assert!(prompt.assign.contains(&"src/main.rs".to_string()));
    }

    #[test]
    fn prompt_serializes_to_yaml() {
        let yaml = PromptBuilder::new("triage", Layer::Deciders)
            .with_paths(["events/"])
            .into_yaml()
            .expect("should serialize");

        assert!(yaml.contains("role: triage"));
        assert!(yaml.contains("layer: deciders"));
        assert!(yaml.contains("assign:"));
    }

    #[test]
    fn generate_prompt_yaml_includes_paths() {
        let paths = vec!["src/foo.rs".to_string()];
        let yaml = generate_prompt_yaml("executor", Layer::Implementers, &paths).unwrap();

        assert!(yaml.contains("assign:"));
        assert!(yaml.contains("src/foo.rs"));
    }

    #[test]
    fn generate_role_yaml_has_correct_structure() {
        let yaml = generate_role_yaml("custom", Layer::Observers);

        assert!(yaml.contains("role: custom"));
        assert!(yaml.contains("layer: observers"));
        assert!(yaml.contains("type: worker"));
    }

    #[test]
    fn generate_prompt_yaml_template_has_correct_structure() {
        let yaml = generate_prompt_yaml_template("custom", Layer::Planners);

        assert!(yaml.contains("role: custom"));
        assert!(yaml.contains("layer: planners"));
        assert!(yaml.contains("prompt:"));
    }
}
